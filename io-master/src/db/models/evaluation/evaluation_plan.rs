//! The evaluation plans (`EvaluationPlan`: a plan, an evaluation copied
//! from it, the report), in the table `evaluation_plans` of the database
//! of the user, with their lists in tables of their own: create, read,
//! update; never deleted (user, 2026-10-28).
//!
//! ```text
//! evaluation_plans            one row per plan: its fields and its state
//! evaluation_plan_standards   (evaluation_plan, position, oscal_standard_document)  oscal_documents
//! evaluation_plan_documents   (evaluation_plan, position, product_document)        product_documents
//! evaluation_plan_verdicts    (evaluation_plan, position, verdict)                 possible_verdicts
//! work_units                  the work units, each naming its plan and its position (work_unit.rs)
//! ```
//!
//! **The work units of a plan are not written with it** (user,
//! 2026-10-28): a plan names them by their ids, and they are made and
//! changed one by one (`work_unit.rs`), so that a change of one is a small
//! transaction; a work unit is taken out of a plan in planning by
//! `delete_work_unit` (user, 2026-10-29). Writing a plan only sets the
//! order of its work units, and must name every one of them.

use rusqlite::{Connection, Row, named_params};

use crate::db::error::DbError;
use crate::db::models::evaluation::lab_proof::{insert_lab_proof, read_lab_proof};
use crate::db::models::evaluation::product_document::{insert_product_document, read_product_document};
use crate::db::models::evaluation::work_unit::{
    MAX_WORK_UNITS, insert_work_unit, read_work_units, verdict_name, verdict_of, write_dependencies,
};
use crate::db::models::lists::{check_length, position, read_ids, write_ids};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::model::evaluation::lab_proof::LabProof;
use crate::model::evaluation::product_document::ProductDocument;
use crate::model::evaluation::work_unit::{Quote, Verdict, WorkUnit};

/// The most plans (with their evaluations and reports) of the user.
pub const MAX_PLANS: usize = 2000;

/// The most standards and documents of the product of one plan.
pub const MAX_STANDARDS: usize = 20;
pub const MAX_DOCUMENTS: usize = 50;

/// The most verdicts a plan knows: every verdict there is.
const MAX_VERDICTS: usize = 3;

/// The tables, made when they are missing (`schema.rs`, `USER_TABLES`), after the
/// tables they name.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS evaluation_plans (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL,
        description TEXT NOT NULL,
        state       TEXT NOT NULL CHECK (state IN ('plan', 'evaluation', 'report')),
        source_plan INTEGER REFERENCES evaluation_plans(id),
        product     TEXT NOT NULL,
        client      TEXT NOT NULL,
        evaluator   TEXT NOT NULL,
        started     INTEGER,
        blocked     INTEGER NOT NULL,
        ai_agent INTEGER REFERENCES ai_agents(id)
    );

    CREATE TABLE IF NOT EXISTS evaluation_plan_standards (
        evaluation_plan         INTEGER NOT NULL REFERENCES evaluation_plans(id),
        position                INTEGER NOT NULL,
        oscal_standard_document INTEGER NOT NULL REFERENCES oscal_standard_documents(id),
        PRIMARY KEY (evaluation_plan, position),
        UNIQUE (evaluation_plan, oscal_standard_document)
    );

    CREATE TABLE IF NOT EXISTS evaluation_plan_documents (
        evaluation_plan  INTEGER NOT NULL REFERENCES evaluation_plans(id),
        position         INTEGER NOT NULL,
        product_document INTEGER NOT NULL REFERENCES product_documents(id),
        PRIMARY KEY (evaluation_plan, position),
        UNIQUE (evaluation_plan, product_document)
    );

    CREATE TABLE IF NOT EXISTS evaluation_plan_verdicts (
        evaluation_plan INTEGER NOT NULL REFERENCES evaluation_plans(id),
        position        INTEGER NOT NULL,
        verdict         TEXT NOT NULL CHECK (verdict IN ('pass', 'fail', 'inconclusive')),
        PRIMARY KEY (evaluation_plan, position),
        UNIQUE (evaluation_plan, verdict)
    );
";

/// The columns every read of a plan selects.
const COLUMNS: &str =
    "id, name, description, state, source_plan, product, client, evaluator, started, blocked, ai_agent";

/// The name of a state in the database.
fn state_name(state: PlanState) -> &'static str {
    match state {
        PlanState::Plan => "plan",
        PlanState::Evaluation => "evaluation",
        PlanState::Report => "report",
    }
}

/// The state of its name; a name the program never writes is
/// `DbError::Corrupt`.
fn state_of(name: &str) -> Result<PlanState, DbError> {
    match name {
        "plan" => Ok(PlanState::Plan),
        "evaluation" => Ok(PlanState::Evaluation),
        "report" => Ok(PlanState::Report),
        _ => Err(DbError::Corrupt { table: "evaluation_plans", column: "state", value: name.to_owned() }),
    }
}

/// Refuses lists longer than their limits, a plan that knows no verdict,
/// and a plan in planning that names a plan it was copied from.
///
/// Called by: [`create_evaluation_plan`], [`update_evaluation_plan`].
fn check_plan(plan: &EvaluationPlan) -> Result<(), DbError> {
    check_length(plan.oscal_documents.len(), MAX_STANDARDS, "standards of a plan")?;
    check_length(plan.product_documents.len(), MAX_DOCUMENTS, "documents of a plan")?;
    check_length(plan.possible_verdicts.len(), MAX_VERDICTS, "verdicts of a plan")?;
    check_length(plan.work_units.len(), MAX_WORK_UNITS, "work units of a plan")?;

    ensure!(!plan.possible_verdicts.is_empty(), DbError::Inconsistent("a plan that knows no verdict".to_owned()));
    ensure!(
        plan.state != PlanState::Plan || plan.source_plan.is_none(),
        DbError::Inconsistent("a plan in planning is copied from no plan".to_owned())
    );

    Ok(())
}

/// Writes the lists of the plan `id` anew: its standards, its documents
/// and its verdicts. Run inside the transaction of its caller.
///
/// Called by: [`create_evaluation_plan`], [`update_evaluation_plan`].
/// Calls: `lists::write_ids`.
fn write_lists(connection: &Connection, id: i64, plan: &EvaluationPlan) -> Result<(), DbError> {
    write_ids(
        connection,
        "DELETE FROM evaluation_plan_standards WHERE evaluation_plan = :owner",
        "INSERT INTO evaluation_plan_standards (evaluation_plan, position, oscal_standard_document)
         VALUES (:owner, :position, :id)",
        id,
        &plan.oscal_documents,
    )?;
    write_ids(
        connection,
        "DELETE FROM evaluation_plan_documents WHERE evaluation_plan = :owner",
        "INSERT INTO evaluation_plan_documents (evaluation_plan, position, product_document)
         VALUES (:owner, :position, :id)",
        id,
        &plan.product_documents,
    )?;

    connection
        .execute("DELETE FROM evaluation_plan_verdicts WHERE evaluation_plan = :id", named_params! { ":id": id })?;

    for (index, verdict) in plan.possible_verdicts.iter().enumerate() {
        connection.execute(
            "INSERT INTO evaluation_plan_verdicts (evaluation_plan, position, verdict) VALUES (:id, :position, :verdict)",
            named_params! { ":id": id, ":position": position(index)?, ":verdict": verdict_name(*verdict) },
        )?;
    }

    Ok(())
}

/// Stores a new plan and gives its id, in one transaction with its lists.
/// A new object has no id yet (`id` 0): the database gives it; a new plan
/// has no work units yet: they are added one by one (`create_work_unit`).
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`insert_plan`].
pub fn create_evaluation_plan(db: &UserDatabase, plan: &EvaluationPlan) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let id = insert_plan(&transaction, plan)?;

    transaction.commit()?;

    Ok(id)
}

/// Writes a new plan with its lists and gives its id, inside the
/// transaction of its caller.
///
/// Called by: [`create_evaluation_plan`], [`create_evaluation`].
/// Calls: [`check_plan`], [`write_lists`].
fn insert_plan(connection: &Connection, plan: &EvaluationPlan) -> Result<i64, DbError> {
    ensure!(plan.id == 0, DbError::Inconsistent("a new plan has an id already".to_owned()));
    ensure!(plan.work_units.is_empty(), DbError::Inconsistent("a new plan has work units already".to_owned()));
    check_plan(plan)?;

    let count: i64 = connection.query_row("SELECT COUNT(*) FROM evaluation_plans", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_PLANS,
        DbError::Inconsistent(format!("more than {MAX_PLANS} plans"))
    );

    connection.execute(
        "INSERT INTO evaluation_plans (name, description, state, source_plan, product, client, evaluator, started,
                                       blocked, ai_agent)
         VALUES (:name, :description, :state, :source_plan, :product, :client, :evaluator, :started, :blocked,
                 :ai_agent)",
        named_params! {
            ":name": plan.name,
            ":description": plan.description,
            ":state": state_name(plan.state),
            ":source_plan": plan.source_plan,
            ":product": plan.product,
            ":client": plan.client,
            ":evaluator": plan.evaluator,
            ":started": plan.started,
            ":blocked": plan.blocked,
            ":ai_agent": plan.ai_agent,
        },
    )?;

    let id = connection.last_insert_rowid();

    write_lists(connection, id, plan)?;

    Ok(id)
}

/// The model of a row of `evaluation_plans`, its lists still empty
/// (`read_lists` fills them).
///
/// Called by: [`read_evaluation_plan`].
fn plan_of(row: &Row<'_>) -> Result<EvaluationPlan, DbError> {
    let state: String = row.get("state")?;

    Ok(EvaluationPlan {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        state: state_of(&state)?,
        source_plan: row.get("source_plan")?,
        product: row.get("product")?,
        client: row.get("client")?,
        evaluator: row.get("evaluator")?,
        started: row.get("started")?,
        oscal_documents: Vec::new(),
        product_documents: Vec::new(),
        possible_verdicts: Vec::new(),
        work_units: Vec::new(),
        blocked: row.get("blocked")?,
        ai_agent: row.get("ai_agent")?,
    })
}

/// Reads the verdicts the plan `id` knows, in their order.
///
/// Called by: [`read_lists`].
fn read_verdicts(connection: &Connection, id: i64) -> Result<Vec<Verdict>, DbError> {
    let mut statement = connection
        .prepare("SELECT verdict FROM evaluation_plan_verdicts WHERE evaluation_plan = :id ORDER BY position")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let mut verdicts = Vec::with_capacity(MAX_VERDICTS);

    for _ in 0..=MAX_VERDICTS {
        let Some(row) = rows.next()? else {
            return Ok(verdicts);
        };
        let name: String = row.get("verdict")?;

        verdicts.push(verdict_of("evaluation_plan_verdicts", "verdict", &name)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_VERDICTS} verdicts of a plan in the database")))
}

/// Fills the lists of `plan` from their tables: its standards, its
/// documents, its verdicts and the ids of its work units.
///
/// Called by: [`read_evaluation_plan`].
/// Calls: `lists::read_ids`, [`read_verdicts`].
fn read_lists(connection: &Connection, plan: &mut EvaluationPlan) -> Result<(), DbError> {
    plan.oscal_documents = read_ids(
        connection,
        "SELECT oscal_standard_document AS id FROM evaluation_plan_standards
         WHERE evaluation_plan = :owner ORDER BY position",
        plan.id,
        MAX_STANDARDS,
    )?;
    plan.product_documents = read_ids(
        connection,
        "SELECT product_document AS id FROM evaluation_plan_documents WHERE evaluation_plan = :owner ORDER BY position",
        plan.id,
        MAX_DOCUMENTS,
    )?;
    plan.possible_verdicts = read_verdicts(connection, plan.id)?;
    plan.work_units = read_ids(
        connection,
        "SELECT id FROM work_units WHERE evaluation_plan = :owner ORDER BY position",
        plan.id,
        MAX_WORK_UNITS,
    )?;

    Ok(())
}

/// Reads the plan `id` with its lists (its work units by their ids).
///
/// Called by: [`read_evaluation_plans`]; the views rewritten on `db/`.
/// Calls: [`plan_of`], [`read_lists`].
pub fn read_evaluation_plan(db: &UserDatabase, id: i64) -> Result<EvaluationPlan, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM evaluation_plans WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "evaluation_plans", id });
    };
    let mut plan = plan_of(row)?;

    read_lists(connection, &mut plan)?;

    Ok(plan)
}

/// Reads every plan, evaluation and report, the newest first.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`read_evaluation_plan`].
pub fn read_evaluation_plans(db: &UserDatabase) -> Result<Vec<EvaluationPlan>, DbError> {
    let mut statement = db.connection().prepare("SELECT id FROM evaluation_plans ORDER BY id DESC")?;
    let mut rows = statement.query([])?;
    let mut plans = Vec::new();

    for _ in 0..=MAX_PLANS {
        let Some(row) = rows.next()? else {
            return Ok(plans);
        };

        plans.push(read_evaluation_plan(db, row.get("id")?)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_PLANS} plans in the database")))
}

/// Gives the work units of the plan `id` the order of `units`, which must
/// name every work unit of the plan once and nothing else (a work unit is
/// removed by `delete_work_unit`). Run inside the transaction of its caller.
///
/// Called by: [`update_evaluation_plan`].
fn order_work_units(connection: &Connection, id: i64, units: &[i64]) -> Result<(), DbError> {
    let stored = read_ids(connection, "SELECT id FROM work_units WHERE evaluation_plan = :owner", id, MAX_WORK_UNITS)?;

    ensure!(
        units.len() == stored.len(),
        DbError::Inconsistent("a plan written without all its work units, or with one twice".to_owned())
    );

    for unit in units {
        ensure!(stored.contains(unit), DbError::Inconsistent(format!("the work unit {unit} is not one of the plan")));
    }

    for (index, unit) in units.iter().enumerate() {
        connection.execute(
            "UPDATE work_units SET position = :position WHERE id = :id",
            named_params! { ":id": unit, ":position": position(index)? },
        )?;
    }

    Ok(())
}

/// Writes the plan `plan.id` anew, with its lists and the order of its
/// work units, in one transaction.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_plan`], [`write_lists`], [`order_work_units`].
pub fn update_evaluation_plan(db: &UserDatabase, plan: &EvaluationPlan) -> Result<(), DbError> {
    check_plan(plan)?;

    let transaction = db.connection().unchecked_transaction()?;
    let changed = transaction.execute(
        "UPDATE evaluation_plans SET name = :name, description = :description, state = :state,
                source_plan = :source_plan, product = :product, client = :client, evaluator = :evaluator,
                started = :started, blocked = :blocked, ai_agent = :ai_agent
         WHERE id = :id",
        named_params! {
            ":id": plan.id,
            ":name": plan.name,
            ":description": plan.description,
            ":state": state_name(plan.state),
            ":source_plan": plan.source_plan,
            ":product": plan.product,
            ":client": plan.client,
            ":evaluator": plan.evaluator,
            ":started": plan.started,
            ":blocked": plan.blocked,
            ":ai_agent": plan.ai_agent,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "evaluation_plans", id: plan.id });

    write_lists(&transaction, plan.id, plan)?;
    order_work_units(&transaction, plan.id, &plan.work_units)?;
    transaction.commit()?;

    Ok(())
}

/// The new id of `old` in the pairs (old id, new id) of a copy.
///
/// Called by: [`copied_unit`], [`create_evaluation`].
fn mapped(pairs: &[(i64, i64)], old: i64) -> Result<i64, DbError> {
    for (from, to) in pairs {
        if *from == old {
            return Ok(*to);
        }
    }

    Err(DbError::Inconsistent(format!("{old} is not copied with its plan")))
}

/// The new ids of `old`, in their order.
///
/// Called by: [`copied_unit`].
fn all_mapped(pairs: &[(i64, i64)], old: &[i64]) -> Result<Vec<i64>, DbError> {
    let mut ids = Vec::with_capacity(old.len());

    for id in old {
        ids.push(mapped(pairs, *id)?);
    }

    Ok(ids)
}

/// The pairs (old id, new id) of what the copy of a plan makes anew: its
/// work units, the places of its documents, the places of its proofs. The
/// three are apart, since their ids come from different tables.
struct Copies {
    units: Vec<(i64, i64)>,
    documents: Vec<(i64, i64)>,
    proofs: Vec<(i64, i64)>,
}

/// The work unit `unit` of a plan as it is copied into an evaluation: no
/// id yet, its documents, proofs and quotes named by their copies (its
/// dependencies are given by the caller, once every copy has its id).
///
/// Called by: [`create_evaluation`].
fn copied_unit(unit: &WorkUnit, copies: &Copies) -> Result<WorkUnit, DbError> {
    let mut quotes = Vec::with_capacity(unit.quotes.len());

    for quote in &unit.quotes {
        quotes.push(Quote {
            product_document: mapped(&copies.documents, quote.product_document)?,
            text: quote.text.clone(),
        });
    }

    Ok(WorkUnit {
        id: 0,
        input_product_documents: all_mapped(&copies.documents, &unit.input_product_documents)?,
        input_functional: match &unit.input_functional {
            Some(proofs) => Some(all_mapped(&copies.proofs, proofs)?),
            None => None,
        },
        quotes,
        ..unit.clone()
    })
}

/// Reserves anew, for the copy of a plan, the places of its documents and
/// of the proofs of its work units: their names, without files or texts.
/// Gives their pairs (old id, new id); the work units are copied later.
///
/// Called by: [`create_evaluation`].
fn copied_places(
    db: &UserDatabase,
    connection: &Connection,
    plan: &EvaluationPlan,
    units: &[WorkUnit],
) -> Result<Copies, DbError> {
    let mut copies = Copies { units: Vec::new(), documents: Vec::new(), proofs: Vec::new() };

    for id in &plan.product_documents {
        let name = read_product_document(db, *id)?.name;
        let place = ProductDocument { id: 0, name, content: None, version: None };

        copies.documents.push((*id, insert_product_document(connection, &place)?));
    }

    for unit in units {
        let Some(proofs) = &unit.input_functional else { continue };

        for id in proofs {
            if mapped(&copies.proofs, *id).is_err() {
                let name = read_lab_proof(db, *id)?.name;
                let place = LabProof { id: 0, name, proof: None, performed: None, tester: None, equipment: None };

                copies.proofs.push((*id, insert_lab_proof(connection, &place)?));
            }
        }
    }

    Ok(copies)
}

/// Begins an evaluation: copies the plan `source` (in planning, neither
/// blocked nor with a cross reference, as `panels/logic/evaluation_plan/order.rs`
/// marks them) in one transaction, as the evaluation of `product` for
/// `client` by `evaluator`, begun at `started`, with the agent `agent` for the
/// work units without one of their own. The copy has its own places
/// of the documents and proofs and its own work units, in the order of the
/// plan: first written without their dependencies, then given them by their
/// new ids, so that the order of the writing does not matter. The plan
/// stays as it was. Gives the id of the evaluation.
///
/// Called by: `panels/logic/evaluation/evaluations.rs` (`new_evaluation`).
/// Calls: [`read_evaluation_plan`], `read_work_units`, [`copied_places`],
/// [`insert_plan`], [`copied_unit`], `insert_work_unit`,
/// `write_dependencies`.
pub fn create_evaluation(
    db: &UserDatabase,
    source: i64,
    product: &str,
    client: &str,
    evaluator: &str,
    started: i64,
    agent: Option<i64>,
) -> Result<i64, DbError> {
    let plan = read_evaluation_plan(db, source)?;
    let units = read_work_units(db, source)?;

    ensure!(plan.state == PlanState::Plan, DbError::Inconsistent("only a plan in planning is copied".to_owned()));
    ensure!(!plan.blocked, DbError::Inconsistent("a blocked plan is not copied".to_owned()));

    for unit in &units {
        ensure!(
            !unit.cross_reference,
            DbError::Inconsistent(format!("a plan with a cross reference is not copied ({})", unit.label))
        );
    }

    let transaction = db.connection().unchecked_transaction()?;
    let mut copies = copied_places(db, &transaction, &plan, &units)?;
    let evaluation = EvaluationPlan {
        id: 0,
        state: PlanState::Evaluation,
        source_plan: Some(source),
        product: product.to_owned(),
        client: client.to_owned(),
        evaluator: evaluator.to_owned(),
        started: Some(started),
        product_documents: all_mapped(&copies.documents, &plan.product_documents)?,
        work_units: Vec::new(),
        blocked: false,
        ai_agent: agent,
        ..plan
    };
    let id = insert_plan(&transaction, &evaluation)?;

    for unit in &units {
        let alone = WorkUnit { dependencies: Vec::new(), ..copied_unit(unit, &copies)? };

        copies.units.push((unit.id, insert_work_unit(&transaction, id, &alone)?));
    }

    for unit in &units {
        write_dependencies(
            &transaction,
            mapped(&copies.units, unit.id)?,
            &all_mapped(&copies.units, &unit.dependencies)?,
        )?;
    }

    transaction.commit()?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Tests of the plans: stored with their lists, read, changed; the
    //! order of their work units, and a work unit taken out of a plan.

    use super::*;

    use crate::db::models::evaluation::lab_proof::create_lab_proof;
    use crate::db::models::evaluation::work_unit::{create_work_unit, read_work_unit, update_work_unit};
    use crate::model::evaluation::work_unit::WorkUnit;
    use crate::test_support::database::{database, plan, unit};

    #[test]
    fn a_plan_is_stored_with_its_lists_and_read_back() {
        let (_directory, db) = database();
        let new = plan(&db);
        let id = create_evaluation_plan(&db, &new).unwrap();

        assert_eq!(read_evaluation_plan(&db, id).unwrap(), EvaluationPlan { id, ..new });
    }

    #[test]
    fn an_evaluation_names_its_plan_and_is_read_among_the_plans() {
        let (_directory, db) = database();
        let source = create_evaluation_plan(&db, &plan(&db)).unwrap();
        let copy = EvaluationPlan {
            state: PlanState::Evaluation,
            source_plan: Some(source),
            product: "Router R7, firmware 2.4.1".to_owned(),
            client: "ACME".to_owned(),
            evaluator: "K. K.".to_owned(),
            started: Some(1_793_000_000),
            possible_verdicts: vec![Verdict::Pass, Verdict::Fail, Verdict::Inconclusive],
            ..plan(&db)
        };
        let id = create_evaluation_plan(&db, &copy).unwrap();
        let plans = read_evaluation_plans(&db).unwrap();

        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0], EvaluationPlan { id, ..copy });
    }

    #[test]
    fn what_does_not_hold_together_is_refused() {
        let (_directory, db) = database();

        assert!(matches!(
            create_evaluation_plan(&db, &EvaluationPlan { id: 3, ..plan(&db) }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_evaluation_plan(&db, &EvaluationPlan { possible_verdicts: Vec::new(), ..plan(&db) }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_evaluation_plan(&db, &EvaluationPlan { source_plan: Some(1), ..plan(&db) }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(read_evaluation_plan(&db, 9), Err(DbError::NoRow { .. })));
        assert!(matches!(
            update_evaluation_plan(&db, &EvaluationPlan { id: 9, ..plan(&db) }),
            Err(DbError::NoRow { .. })
        ));
    }

    #[test]
    fn writing_a_plan_orders_its_work_units_and_must_name_them_all() {
        let (_directory, db) = database();
        let new = plan(&db);
        let id = create_evaluation_plan(&db, &new).unwrap();
        let first = create_work_unit(&db, id, &unit(&new, "ASE_INT.1-1")).unwrap();
        let second = create_work_unit(&db, id, &unit(&new, "ASE_INT.1-2")).unwrap();
        let third = create_work_unit(&db, id, &unit(&new, "ASE_INT.1-3")).unwrap();
        let mut stored = read_evaluation_plan(&db, id).unwrap();

        assert_eq!(stored.work_units, vec![first, second, third]);

        stored.work_units = vec![third, first, second];
        update_evaluation_plan(&db, &stored).unwrap();

        assert_eq!(read_evaluation_plan(&db, id).unwrap().work_units, vec![third, first, second]);

        stored.work_units = vec![third, second];

        assert!(matches!(update_evaluation_plan(&db, &stored), Err(DbError::Inconsistent(_))));
        assert!(read_work_unit(&db, first).is_ok());
    }

    #[test]
    fn a_plan_does_not_take_a_work_unit_of_another() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let one = create_evaluation_plan(&db, &routers).unwrap();
        let other = create_evaluation_plan(&db, &routers).unwrap();
        let unit = create_work_unit(&db, other, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let stored = read_evaluation_plan(&db, one).unwrap();

        assert!(matches!(
            update_evaluation_plan(&db, &EvaluationPlan { work_units: vec![unit], ..stored }),
            Err(DbError::Inconsistent(_))
        ));
    }

    /// A plan with two work units, the first depending on the second (a
    /// dependency set after both were made), the second with a proof and a
    /// quote of the document of the plan.
    fn plan_with_units(db: &UserDatabase) -> (i64, i64, i64) {
        let routers = plan(db);
        let id = create_evaluation_plan(db, &routers).unwrap();
        let proof =
            LabProof { id: 0, name: "Scan".to_owned(), proof: None, performed: None, tester: None, equipment: None };
        let second = WorkUnit {
            input_functional: Some(vec![create_lab_proof(db, &proof).unwrap()]),
            quotes: vec![Quote { product_document: routers.product_documents[0], text: "The TOE".to_owned() }],
            ..unit(&routers, "ATE_IND.1-1")
        };
        let first = create_work_unit(db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let second = create_work_unit(db, id, &second).unwrap();
        let mut stored = read_work_unit(db, first).unwrap();

        stored.dependencies = vec![second];
        update_work_unit(db, &stored).unwrap();

        (id, first, second)
    }

    #[test]
    fn an_evaluation_is_a_copy_of_its_plan_with_its_own_places_and_work_units() {
        let (_directory, db) = database();
        let (source, first, second) = plan_with_units(&db);
        let integration =
            crate::db::models::ai::integration::create_integration(&db, &crate::test_support::database::integration())
                .unwrap();
        let planner =
            crate::db::models::ai::agent::create_agent(&db, &crate::test_support::database::agent(integration))
                .unwrap();

        update_evaluation_plan(
            &db,
            &EvaluationPlan { ai_agent: Some(planner), ..read_evaluation_plan(&db, source).unwrap() },
        )
        .unwrap();

        let plan_before = read_evaluation_plan(&db, source).unwrap();
        let id = create_evaluation(&db, source, "Router R7", "ACME", "K. K.", 1_793_000_000, Some(planner)).unwrap();
        let evaluation = read_evaluation_plan(&db, id).unwrap();
        let units = read_work_units(&db, id).unwrap();

        assert_eq!(read_evaluation_plan(&db, source).unwrap(), plan_before);
        assert_eq!(evaluation.state, PlanState::Evaluation);
        assert_eq!(evaluation.source_plan, Some(source));
        // The evaluation has the agent given when it begins.
        assert_eq!((plan_before.ai_agent, evaluation.ai_agent), (Some(planner), Some(planner)));
        assert_eq!(evaluation.oscal_documents, plan_before.oscal_documents);
        assert_ne!(evaluation.product_documents, plan_before.product_documents);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].label, "ASE_INT.1-1");
        assert!(!evaluation.work_units.contains(&first) && !evaluation.work_units.contains(&second));
        assert_eq!(units[0].dependencies, vec![units[1].id]);
        assert_eq!(units[1].input_product_documents, evaluation.product_documents);
        assert_eq!(units[1].quotes[0].product_document, evaluation.product_documents[0]);
        assert_ne!(units[1].input_functional, read_work_unit(&db, second).unwrap().input_functional);
    }

    #[test]
    fn a_dependency_on_a_later_work_unit_is_copied_and_a_marked_plan_is_not() {
        let (_directory, db) = database();
        let (source, first, _second) = plan_with_units(&db);
        let evaluation = create_evaluation(&db, source, "R7", "ACME", "K. K.", 0, None).unwrap();

        assert_eq!(read_work_units(&db, evaluation).unwrap().len(), 2);

        let mut crossed = read_work_unit(&db, first).unwrap();

        crossed.cross_reference = true;
        update_work_unit(&db, &crossed).unwrap();

        assert!(matches!(
            create_evaluation(&db, source, "R7", "ACME", "K. K.", 0, None),
            Err(DbError::Inconsistent(_))
        ));

        crossed.cross_reference = false;
        update_work_unit(&db, &crossed).unwrap();

        let blocked = EvaluationPlan { blocked: true, ..read_evaluation_plan(&db, source).unwrap() };

        update_evaluation_plan(&db, &blocked).unwrap();

        assert!(matches!(
            create_evaluation(&db, source, "R7", "ACME", "K. K.", 0, None),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            create_evaluation(&db, evaluation, "R8", "ACME", "K. K.", 0, None),
            Err(DbError::Inconsistent(_))
        ));
    }
}
