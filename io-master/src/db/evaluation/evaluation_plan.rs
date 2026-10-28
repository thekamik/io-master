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
//! transaction. Writing a plan only sets their order; **a work unit the
//! plan is written without leaves it**: its row is removed, with its lists
//! and the dependencies of the others on it. That is the only removal of
//! `db/` (no delete, user 2026-10-28), and how a planner takes out a work
//! unit added by mistake.

#![allow(dead_code, reason = "the new database (2026-10-28), not used by the views yet, only by its tests")]

use rusqlite::{Connection, Row, named_params};

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::db::evaluation::lab_proof::{insert_lab_proof, read_lab_proof};
use crate::db::evaluation::product_document::{insert_product_document, read_product_document};
use crate::db::evaluation::work_unit::{MAX_WORK_UNITS, insert_work_unit, read_work_units, verdict_name, verdict_of};
use crate::db::lists::{check_length, position, read_ids, write_ids};
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

/// The tables, made when they are missing (`database.rs`), after the
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
        blocked     INTEGER NOT NULL
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
const COLUMNS: &str = "id, name, description, state, source_plan, product, client, evaluator, started, blocked";

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
pub fn create_evaluation_plan(db: &Database, plan: &EvaluationPlan) -> Result<i64, DbError> {
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
                                       blocked)
         VALUES (:name, :description, :state, :source_plan, :product, :client, :evaluator, :started, :blocked)",
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
pub fn read_evaluation_plan(db: &Database, id: i64) -> Result<EvaluationPlan, DbError> {
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
pub fn read_evaluation_plans(db: &Database) -> Result<Vec<EvaluationPlan>, DbError> {
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

/// Gives the work units of the plan `id` the order of `units`, and removes
/// those `units` does not name (see the head of this file). Every id of
/// `units` must be a work unit of the plan. Run inside the transaction of
/// its caller.
///
/// Called by: [`update_evaluation_plan`].
fn order_work_units(connection: &Connection, id: i64, units: &[i64]) -> Result<(), DbError> {
    let stored = read_ids(connection, "SELECT id FROM work_units WHERE evaluation_plan = :owner", id, MAX_WORK_UNITS)?;

    for unit in units {
        ensure!(stored.contains(unit), DbError::Inconsistent(format!("the work unit {unit} is not one of the plan")));
    }

    for unit in &stored {
        if !units.contains(unit) {
            connection.execute("DELETE FROM work_units WHERE id = :id", named_params! { ":id": unit })?;
        }
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
/// work units, in one transaction; a work unit it does not name leaves
/// it (see the head of this file).
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_plan`], [`write_lists`], [`order_work_units`].
pub fn update_evaluation_plan(db: &Database, plan: &EvaluationPlan) -> Result<(), DbError> {
    check_plan(plan)?;

    let transaction = db.connection().unchecked_transaction()?;
    let changed = transaction.execute(
        "UPDATE evaluation_plans SET name = :name, description = :description, state = :state,
                source_plan = :source_plan, product = :product, client = :client, evaluator = :evaluator,
                started = :started, blocked = :blocked
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
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "evaluation_plans", id: plan.id });

    write_lists(&transaction, plan.id, plan)?;
    order_work_units(&transaction, plan.id, &plan.work_units)?;
    transaction.commit()?;

    Ok(())
}

/// The order in which the work units of a plan can be carried out: each
/// after all its dependencies, otherwise in the order of the plan (Kahn's
/// algorithm, without recursion: each work unit is taken once, and each
/// taking looks once at every work unit). Gives the positions in `units`;
/// a cross reference (a cycle of dependencies) leaves work units that can
/// never be taken, and is refused with their labels.
///
/// Called by: [`create_evaluation`], which checks a plan again before it
/// is copied (user, 2026-10-28).
fn work_order(units: &[WorkUnit]) -> Result<Vec<usize>, DbError> {
    let mut waiting: Vec<usize> = Vec::with_capacity(units.len());

    for unit in units {
        waiting.push(unit.dependencies.len());
    }

    let mut order: Vec<usize> = Vec::with_capacity(units.len());

    for (index, count) in waiting.iter().enumerate() {
        if *count == 0 {
            order.push(index);
        }
    }

    for taken in 0..units.len() {
        let Some(&index) = order.get(taken) else { break };
        let Some(taken_unit) = units.get(index) else { break };
        let id = taken_unit.id;

        for (other, unit) in units.iter().enumerate() {
            if unit.dependencies.contains(&id)
                && let Some(count) = waiting.get_mut(other)
            {
                *count = count.saturating_sub(1);

                if *count == 0 {
                    order.push(other);
                }
            }
        }
    }

    ensure!(
        order.len() == units.len(),
        DbError::Inconsistent(format!("a cross reference: {}", left_over(units, &order)))
    );

    Ok(order)
}

/// The labels of the work units `order` does not hold: those in a cycle of
/// dependencies, and those behind it.
///
/// Called by: [`work_order`].
fn left_over(units: &[WorkUnit], order: &[usize]) -> String {
    let mut labels = Vec::new();

    for (index, unit) in units.iter().enumerate() {
        if !order.contains(&index) {
            labels.push(unit.label.as_str());
        }
    }

    labels.join(", ")
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
/// id yet, its dependencies, documents, proofs and quotes named by their
/// copies.
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
        dependencies: all_mapped(&copies.units, &unit.dependencies)?,
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
    db: &Database,
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

/// Begins an evaluation: copies the plan `source` (in planning, not
/// blocked, checked again for cross references) in one transaction, as
/// the evaluation of `product` for `client` by `evaluator`, begun at
/// `started`. The copy has its own places of the documents and proofs and
/// its own work units, in the order of the plan, their dependencies on
/// each other; the plan stays as it was. Gives the id of the evaluation.
///
/// Called by: nothing yet (the view of the evaluation, user 2026-10-28).
/// Calls: [`read_evaluation_plan`], `read_work_units`, [`work_order`],
/// [`copied_places`], [`insert_plan`], [`copied_unit`], `insert_work_unit`,
/// [`order_work_units`].
pub fn create_evaluation(
    db: &Database,
    source: i64,
    product: &str,
    client: &str,
    evaluator: &str,
    started: i64,
) -> Result<i64, DbError> {
    let plan = read_evaluation_plan(db, source)?;
    let units = read_work_units(db, source)?;

    ensure!(plan.state == PlanState::Plan, DbError::Inconsistent("only a plan in planning is copied".to_owned()));
    ensure!(!plan.blocked, DbError::Inconsistent("a blocked plan is not copied".to_owned()));

    let order = work_order(&units)?;
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
        ..plan
    };
    let id = insert_plan(&transaction, &evaluation)?;

    for index in order {
        let Some(unit) = units.get(index) else { continue };
        let new = insert_work_unit(&transaction, id, &copied_unit(unit, &copies)?)?;

        copies.units.push((unit.id, new));
    }

    let mut in_plan_order = Vec::with_capacity(units.len());

    for unit in &units {
        in_plan_order.push(mapped(&copies.units, unit.id)?);
    }

    order_work_units(&transaction, id, &in_plan_order)?;
    transaction.commit()?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Tests of the plans: stored with their lists, read, changed; the
    //! order of their work units, and a work unit taken out of a plan.

    use super::*;

    use crate::db::evaluation::lab_proof::create_lab_proof;
    use crate::db::evaluation::work_unit::{create_work_unit, read_work_unit, update_work_unit};
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
    fn writing_a_plan_orders_its_work_units_and_takes_out_those_it_does_not_name() {
        let (_directory, db) = database();
        let new = plan(&db);
        let id = create_evaluation_plan(&db, &new).unwrap();
        let first = create_work_unit(&db, id, &unit(&new, "ASE_INT.1-1")).unwrap();
        let second = create_work_unit(&db, id, &unit(&new, "ASE_INT.1-2")).unwrap();
        let third =
            create_work_unit(&db, id, &WorkUnit { dependencies: vec![first], ..unit(&new, "ASE_INT.1-3") }).unwrap();
        let mut stored = read_evaluation_plan(&db, id).unwrap();

        assert_eq!(stored.work_units, vec![first, second, third]);

        stored.work_units = vec![third, second];
        update_evaluation_plan(&db, &stored).unwrap();

        assert_eq!(read_evaluation_plan(&db, id).unwrap().work_units, vec![third, second]);
        assert!(matches!(read_work_unit(&db, first), Err(DbError::NoRow { .. })));
        assert!(read_work_unit(&db, third).unwrap().dependencies.is_empty());
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
    fn plan_with_units(db: &Database) -> (i64, i64, i64) {
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
        let plan_before = read_evaluation_plan(&db, source).unwrap();
        let id = create_evaluation(&db, source, "Router R7", "ACME", "K. K.", 1_793_000_000).unwrap();
        let evaluation = read_evaluation_plan(&db, id).unwrap();
        let units = read_work_units(&db, id).unwrap();

        assert_eq!(read_evaluation_plan(&db, source).unwrap(), plan_before);
        assert_eq!(evaluation.state, PlanState::Evaluation);
        assert_eq!(evaluation.source_plan, Some(source));
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
    fn a_cross_reference_a_blocked_plan_and_an_evaluation_are_not_copied() {
        let (_directory, db) = database();
        let (source, first, second) = plan_with_units(&db);
        let mut back = read_work_unit(&db, second).unwrap();

        back.dependencies = vec![first];
        update_work_unit(&db, &back).unwrap();

        let refused = create_evaluation(&db, source, "R7", "ACME", "K. K.", 0).unwrap_err().to_string();

        assert!(refused.contains("cross reference") && refused.contains("ASE_INT.1-1"), "{refused}");

        let blocked = EvaluationPlan { blocked: true, ..read_evaluation_plan(&db, source).unwrap() };

        update_evaluation_plan(&db, &blocked).unwrap();

        assert!(matches!(create_evaluation(&db, source, "R7", "ACME", "K. K.", 0), Err(DbError::Inconsistent(_))));

        let other = create_evaluation_plan(&db, &plan(&db)).unwrap();
        let evaluation = create_evaluation(&db, other, "R7", "ACME", "K. K.", 0).unwrap();

        assert!(matches!(create_evaluation(&db, evaluation, "R8", "ACME", "K. K.", 0), Err(DbError::Inconsistent(_))));
    }
}
