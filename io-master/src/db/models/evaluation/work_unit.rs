//! The work units of the plans (`WorkUnit`), in the table `work_units` of
//! the database of the user, with their lists in tables of their own:
//! create, read, update, and **delete only from a plan in planning** (user,
//! 2026-10-29): while a plan is planned, the planner changes it freely, and
//! a work unit marked inactive instead of removed would fill the database;
//! once the plan is an evaluation or a report, its work units stay
//! (`delete_work_unit`, the one deletion of `db/`).
//!
//! ```text
//! work_units               one row per work unit: its plan, its position in the plan, its fields;
//!                          functional = 0 when input_functional is None
//! work_unit_dependencies   (work_unit, position, depends_on)       the dependencies
//! work_unit_documents      (work_unit, position, product_document) input_product_documents
//! work_unit_proofs         (work_unit, position, lab_proof)        input_functional
//! work_unit_quotes         (work_unit, position, product_document, text)  the quotes
//! ```
//!
//! What a work unit names must hold together with its plan (checked at
//! every write, [`check_links`]): its standard is one of the standards of
//! the plan, its documents are documents of the plan, its dependencies
//! are work units of the plan, its verdict is one the plan knows, and
//! each quote is taken from one of its documents. **The planning of a work
//! unit of an evaluation is never changed** (user, 2026-10-30): once its plan
//! is no more in planning, only its results are written ([`check_frozen`]).

use rusqlite::{Connection, Row, named_params};

use crate::db::error::DbError;
use crate::db::models::lists::{check_length, position, read_id_list, write_id_list};
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::work_unit::{Author, Quote, Verdict, WorkUnit};

/// The most work units of one plan (the CEM has 813).
pub const MAX_WORK_UNITS: usize = 5000;

/// The most dependencies, documents, proofs and quotes of one work unit.
pub const MAX_DEPENDENCIES: usize = 100;
pub const MAX_DOCUMENTS: usize = 50;
pub const MAX_PROOFS: usize = 50;
pub const MAX_QUOTES: usize = 100;

/// The tables, made when they are missing (`schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS work_units (
        id                  INTEGER PRIMARY KEY,
        evaluation_plan     INTEGER NOT NULL REFERENCES evaluation_plans(id),
        position            INTEGER NOT NULL,
        reference           INTEGER NOT NULL REFERENCES oscal_standard_documents(id),
        label               TEXT NOT NULL,
        oscal_key           TEXT NOT NULL,
        requirement         TEXT NOT NULL,
        guidance            TEXT NOT NULL,
        cross_reference     INTEGER NOT NULL,
        functional          INTEGER NOT NULL,
        ai_agent            INTEGER REFERENCES ai_agents(id),
        findings            TEXT NOT NULL,
        analysis            TEXT NOT NULL,
        vulnerability_notes TEXT NOT NULL,
        notes               TEXT NOT NULL,
        verdict             TEXT CHECK (verdict IN ('pass', 'fail', 'inconclusive')),
        author              TEXT CHECK (author IN ('ai', 'user')),
        UNIQUE (evaluation_plan, reference, oscal_key)
    );

    CREATE TABLE IF NOT EXISTS work_unit_dependencies (
        work_unit  INTEGER NOT NULL REFERENCES work_units(id) ON DELETE CASCADE,
        position   INTEGER NOT NULL,
        depends_on INTEGER NOT NULL REFERENCES work_units(id) ON DELETE CASCADE,
        PRIMARY KEY (work_unit, position),
        UNIQUE (work_unit, depends_on),
        CHECK (work_unit <> depends_on)
    );

    CREATE TABLE IF NOT EXISTS work_unit_documents (
        work_unit        INTEGER NOT NULL REFERENCES work_units(id) ON DELETE CASCADE,
        position         INTEGER NOT NULL,
        product_document INTEGER NOT NULL REFERENCES product_documents(id),
        PRIMARY KEY (work_unit, position),
        UNIQUE (work_unit, product_document)
    );

    CREATE TABLE IF NOT EXISTS work_unit_proofs (
        work_unit INTEGER NOT NULL REFERENCES work_units(id) ON DELETE CASCADE,
        position  INTEGER NOT NULL,
        lab_proof INTEGER NOT NULL REFERENCES lab_proofs(id),
        PRIMARY KEY (work_unit, position),
        UNIQUE (work_unit, lab_proof)
    );

    CREATE TABLE IF NOT EXISTS work_unit_quotes (
        work_unit        INTEGER NOT NULL REFERENCES work_units(id) ON DELETE CASCADE,
        position         INTEGER NOT NULL,
        product_document INTEGER NOT NULL REFERENCES product_documents(id),
        text             TEXT NOT NULL,
        PRIMARY KEY (work_unit, position)
    );
";

/// The columns every read of a work unit selects.
const COLUMNS: &str = "id, reference, label, oscal_key, requirement, guidance, cross_reference, functional, ai_agent,
                       findings, analysis, vulnerability_notes, notes, verdict, author";

/// The name of a verdict in the database (`Verdict::Fail` → "fail").
///
/// Called by: [`check_links`], [`insert_work_unit`], [`update_work_unit`];
/// `evaluation_plan.rs`: `write_lists` (the verdicts of a plan).
pub(in crate::db) fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Inconclusive => "inconclusive",
    }
}

/// The verdict of its name ("fail" → `Verdict::Fail`); a name the program
/// never writes is `DbError::Corrupt`, naming the table and the column it
/// was read from.
///
/// Called by: [`unit_of`]; `evaluation_plan.rs`: `read_verdicts`.
pub(in crate::db) fn verdict_of(table: &'static str, column: &'static str, name: &str) -> Result<Verdict, DbError> {
    match name {
        "pass" => Ok(Verdict::Pass),
        "fail" => Ok(Verdict::Fail),
        "inconclusive" => Ok(Verdict::Inconclusive),
        _ => Err(DbError::Corrupt { table, column, value: name.to_owned() }),
    }
}

/// The name of an author in the column `author` (`Author::User` → "user").
///
/// Called by: [`insert_work_unit`], [`update_work_unit`].
fn author_name(author: Author) -> &'static str {
    match author {
        Author::Ai => "ai",
        Author::User => "user",
    }
}

/// The author of its name in the column `author` ("user" → `Author::User`); a
/// name the program never writes is `DbError::Corrupt`.
///
/// Called by: [`unit_of`].
fn author_of(name: &str) -> Result<Author, DbError> {
    match name {
        "ai" => Ok(Author::Ai),
        "user" => Ok(Author::User),
        _ => Err(DbError::Corrupt { table: "work_units", column: "author", value: name.to_owned() }),
    }
}

/// Refuses a work unit whose lists do not hold together: longer than their
/// limits, a dependency on itself, or a quote of a document it does not
/// use.
///
/// Input: `unit`, the work unit to be stored.
///
/// Output: nothing when it is good; otherwise `DbError::Inconsistent` naming
/// the work unit ("the work unit ADV_ARC.1-1 depends on itself").
///
/// Called by: [`insert_work_unit`], [`update_work_unit`].
/// Calls: `lists.rs`: `check_length`.
fn check_lists(unit: &WorkUnit) -> Result<(), DbError> {
    check_length(unit.dependencies.len(), MAX_DEPENDENCIES, "dependencies of a work unit")?;
    check_length(unit.input_product_documents.len(), MAX_DOCUMENTS, "documents of a work unit")?;
    check_length(unit.input_functional.as_ref().map_or(0, Vec::len), MAX_PROOFS, "proofs of a work unit")?;
    check_length(unit.quotes.len(), MAX_QUOTES, "quotes of a work unit")?;

    ensure!(
        !unit.dependencies.contains(&unit.id),
        DbError::Inconsistent(format!("the work unit {} depends on itself", unit.label))
    );

    for quote in &unit.quotes {
        ensure!(
            unit.input_product_documents.contains(&quote.product_document),
            DbError::Inconsistent(format!("a quote of {} from a document it does not use", unit.label))
        );
    }

    Ok(())
}

/// Refuses a work unit that does not hold together with its plan: its
/// standard, its documents, its dependencies and its verdict must be the
/// plan's.
///
/// Input: `connection`, the open database; `plan`, the id of its plan;
/// `unit`, the work unit to be stored.
///
/// Output: nothing when it is good; otherwise `DbError::Inconsistent` naming
/// what is not of the plan ("a document of ADV_ARC.1-1 is not one of its
/// plan").
///
/// Called by: [`insert_work_unit`], [`update_work_unit`].
/// Calls: `lists.rs`: `read_id_list`; [`verdict_name`].
fn check_links(connection: &Connection, plan: i64, unit: &WorkUnit) -> Result<(), DbError> {
    let standards = read_id_list(
        connection,
        "SELECT oscal_standard_document AS id FROM evaluation_plan_standards WHERE evaluation_plan = :owner",
        plan,
        crate::db::models::evaluation::evaluation_plan::MAX_STANDARDS,
    )?;
    let documents = read_id_list(
        connection,
        "SELECT product_document AS id FROM evaluation_plan_documents WHERE evaluation_plan = :owner",
        plan,
        crate::db::models::evaluation::evaluation_plan::MAX_DOCUMENTS,
    )?;
    let units =
        read_id_list(connection, "SELECT id FROM work_units WHERE evaluation_plan = :owner", plan, MAX_WORK_UNITS)?;

    ensure!(
        standards.contains(&unit.reference),
        DbError::Inconsistent(format!("the standard of {} is not one of its plan", unit.label))
    );

    for document in &unit.input_product_documents {
        ensure!(
            documents.contains(document),
            DbError::Inconsistent(format!("a document of {} is not one of its plan", unit.label))
        );
    }

    for dependency in &unit.dependencies {
        ensure!(
            units.contains(dependency),
            DbError::Inconsistent(format!("a dependency of {} is no work unit of its plan", unit.label))
        );
    }

    if let Some(verdict) = unit.verdict {
        let known: i64 = connection.query_row(
            "SELECT COUNT(*) FROM evaluation_plan_verdicts WHERE evaluation_plan = :plan AND verdict = :verdict",
            named_params! { ":plan": plan, ":verdict": verdict_name(verdict) },
            |row| row.get(0),
        )?;

        ensure!(known == 1, DbError::Inconsistent(format!("the verdict of {} is not one its plan knows", unit.label)));
    }

    Ok(())
}

/// Writes the lists of the work unit `id` anew: its dependencies, its
/// documents and its proofs (lists of ids) and its quotes (a list of
/// values). Run inside the transaction of its caller.
///
/// Input: `connection`, the open database inside a transaction; `id`, the id
/// of the work unit; `unit`, its lists, checked by [`check_lists`] and
/// [`check_links`].
///
/// Output: nothing; an error of SQLite when what a list names is not stored
/// (foreign key) or comes twice (`UNIQUE`).
///
/// Called by: [`insert_work_unit`], [`update_work_unit`].
/// Calls: `lists.rs`: `write_id_list`, `position`.
fn write_lists(connection: &Connection, id: i64, unit: &WorkUnit) -> Result<(), DbError> {
    write_id_list(
        connection,
        "DELETE FROM work_unit_dependencies WHERE work_unit = :owner",
        "INSERT INTO work_unit_dependencies (work_unit, position, depends_on) VALUES (:owner, :position, :id)",
        id,
        &unit.dependencies,
    )?;
    write_id_list(
        connection,
        "DELETE FROM work_unit_documents WHERE work_unit = :owner",
        "INSERT INTO work_unit_documents (work_unit, position, product_document) VALUES (:owner, :position, :id)",
        id,
        &unit.input_product_documents,
    )?;
    write_id_list(
        connection,
        "DELETE FROM work_unit_proofs WHERE work_unit = :owner",
        "INSERT INTO work_unit_proofs (work_unit, position, lab_proof) VALUES (:owner, :position, :id)",
        id,
        unit.input_functional.as_deref().unwrap_or_default(),
    )?;

    connection.execute("DELETE FROM work_unit_quotes WHERE work_unit = :id", named_params! { ":id": id })?;

    for (index, quote) in unit.quotes.iter().enumerate() {
        connection.execute(
            "INSERT INTO work_unit_quotes (work_unit, position, product_document, text)
             VALUES (:id, :position, :document, :text)",
            named_params! {
                ":id": id,
                ":position": position(index)?,
                ":document": quote.product_document,
                ":text": quote.text,
            },
        )?;
    }

    Ok(())
}

/// Writes the dependencies of the work unit `id` anew, inside the
/// transaction of its caller: the copy of a plan gives its work units their
/// dependencies once every copy has its id.
///
/// Input: `connection`, the open database inside a transaction; `id`, the id
/// of the work unit (41); `dependencies`, the ids it depends on (`[40]`).
///
/// Output: nothing; an error when there are more than [`MAX_DEPENDENCIES`],
/// when it depends on itself, or when a dependency is not stored.
///
/// Called by: `evaluation_plan.rs`: `create_evaluation`.
/// Calls: `lists.rs`: `check_length`, `write_id_list`.
pub(in crate::db) fn write_dependencies(connection: &Connection, id: i64, dependencies: &[i64]) -> Result<(), DbError> {
    check_length(dependencies.len(), MAX_DEPENDENCIES, "dependencies of a work unit")?;
    ensure!(!dependencies.contains(&id), DbError::Inconsistent("a work unit that depends on itself".to_owned()));

    write_id_list(
        connection,
        "DELETE FROM work_unit_dependencies WHERE work_unit = :owner",
        "INSERT INTO work_unit_dependencies (work_unit, position, depends_on) VALUES (:owner, :position, :id)",
        id,
        dependencies,
    )
}

/// Stores a new work unit as the last of its plan, with its lists, and
/// gives its id, in one transaction.
///
/// Input: `db`, the open database of the user; `plan`, the id of its plan;
/// `unit`, the new work unit, its `id` 0 (the database gives it).
///
/// Output: the id of the work unit (12); an error as [`insert_work_unit`],
/// and nothing is then written.
///
/// Called by: `panels/logic/evaluation_plan/work_units.rs`:
/// `add_all_work_units`, `add_work_unit`.
/// Calls: [`insert_work_unit`].
pub fn create_work_unit(db: &UserDatabase, plan: i64, unit: &WorkUnit) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let id = insert_work_unit(&transaction, plan, unit)?;

    transaction.commit()?;

    Ok(id)
}

/// Writes a new work unit as the last of its plan, with its lists, and
/// gives its id, inside the transaction of its caller, so that the copy of
/// a plan writes its work units in its own.
///
/// Input: `connection`, the open database inside a transaction; `plan`, the
/// id of its plan; `unit`, the new work unit, its `id` 0.
///
/// Output: the id of the work unit. An error when it has an id, when it is
/// refused ([`check_lists`], [`check_links`]), when the plan has
/// [`MAX_WORK_UNITS`] already, or when its control is in the plan already
/// (`UNIQUE`).
///
/// Called by: [`create_work_unit`]; `evaluation_plan.rs`: `create_evaluation`.
/// Calls: [`check_lists`], [`check_links`], [`verdict_name`], [`author_name`],
/// [`write_lists`].
pub(in crate::db) fn insert_work_unit(connection: &Connection, plan: i64, unit: &WorkUnit) -> Result<i64, DbError> {
    ensure!(unit.id == 0, DbError::Inconsistent("a new work unit has an id already".to_owned()));
    check_lists(unit)?;
    check_links(connection, plan, unit)?;

    // The new work unit is the last of its plan: its position is the
    // number of the work units before it.
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM work_units WHERE evaluation_plan = :plan",
        named_params! { ":plan": plan },
        |row| row.get(0),
    )?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_WORK_UNITS,
        DbError::Inconsistent(format!("more than {MAX_WORK_UNITS} work units in a plan"))
    );

    connection.execute(
        "INSERT INTO work_units (evaluation_plan, position, reference, label, oscal_key, requirement, guidance,
                                 cross_reference, functional, ai_agent, findings, analysis, vulnerability_notes,
                                 notes, verdict, author)
         VALUES (:plan, :position, :reference, :label, :oscal_key, :requirement, :guidance, :cross_reference,
                 :functional, :ai_agent, :findings, :analysis, :vulnerability_notes, :notes, :verdict, :author)",
        named_params! {
            ":plan": plan,
            ":position": count,
            ":reference": unit.reference,
            ":label": unit.label,
            ":oscal_key": unit.oscal_key,
            ":requirement": unit.requirement,
            ":guidance": unit.guidance,
            ":cross_reference": unit.cross_reference,
            ":functional": unit.input_functional.is_some(),
            ":ai_agent": unit.ai_agent,
            ":findings": unit.findings,
            ":analysis": unit.analysis,
            ":vulnerability_notes": unit.vulnerability_notes,
            ":notes": unit.notes,
            ":verdict": unit.verdict.map(verdict_name),
            ":author": unit.author.map(author_name),
        },
    )?;

    let id = connection.last_insert_rowid();

    write_lists(connection, id, unit)?;

    Ok(id)
}

/// The model of one row of `work_units`, its lists still empty
/// ([`read_lists`] fills them). The column `functional` tells a work unit of
/// the documentation (`input_functional` `None`) from a functional test
/// (an empty list, filled by [`read_lists`]).
///
/// Called by: [`read_work_unit`].
/// Calls: [`verdict_of`], [`author_of`].
fn unit_of(row: &Row<'_>) -> Result<WorkUnit, DbError> {
    let verdict: Option<String> = row.get("verdict")?;
    let author: Option<String> = row.get("author")?;
    let functional: bool = row.get("functional")?;

    Ok(WorkUnit {
        id: row.get("id")?,
        reference: row.get("reference")?,
        label: row.get("label")?,
        oscal_key: row.get("oscal_key")?,
        requirement: row.get("requirement")?,
        guidance: row.get("guidance")?,
        dependencies: Vec::new(),
        cross_reference: row.get("cross_reference")?,
        input_product_documents: Vec::new(),
        input_functional: functional.then(Vec::new),
        ai_agent: row.get("ai_agent")?,
        findings: row.get("findings")?,
        analysis: row.get("analysis")?,
        vulnerability_notes: row.get("vulnerability_notes")?,
        notes: row.get("notes")?,
        quotes: Vec::new(),
        verdict: match verdict {
            Some(name) => Some(verdict_of("work_units", "verdict", &name)?),
            None => None,
        },
        author: match author {
            Some(name) => Some(author_of(&name)?),
            None => None,
        },
    })
}

/// Reads the quotes of the work unit `id`, in their order.
///
/// Input: `connection`, the open database; `id`, the id of the work unit.
///
/// Output: the quotes, each with its document (`[Quote { product_document:
/// 3, text: "The TOE ..." }]`); an error when there are more than
/// [`MAX_QUOTES`] (a database the program did not write).
///
/// Called by: [`read_lists`].
fn read_quotes(connection: &Connection, id: i64) -> Result<Vec<Quote>, DbError> {
    let mut statement = connection
        .prepare("SELECT product_document, text FROM work_unit_quotes WHERE work_unit = :id ORDER BY position")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let mut quotes = Vec::new();

    for _ in 0..=MAX_QUOTES {
        let Some(row) = rows.next()? else {
            return Ok(quotes);
        };

        quotes.push(Quote { product_document: row.get("product_document")?, text: row.get("text")? });
    }

    Err(DbError::Inconsistent(format!("more than {MAX_QUOTES} quotes of a work unit in the database")))
}

/// Fills the lists of a work unit from their tables: its dependencies, its
/// documents, its proofs (only for a functional test) and its quotes.
///
/// Input: `connection`, the open database; `unit`, the work unit read from
/// its row, its lists empty.
///
/// Output: nothing; the lists of `unit` are filled. An error when a list is
/// longer than its limit (a database the program did not write).
///
/// Called by: [`read_work_unit`].
/// Calls: `lists.rs`: `read_id_list`; [`read_quotes`].
fn read_lists(connection: &Connection, unit: &mut WorkUnit) -> Result<(), DbError> {
    unit.dependencies = read_id_list(
        connection,
        "SELECT depends_on AS id FROM work_unit_dependencies WHERE work_unit = :owner ORDER BY position",
        unit.id,
        MAX_DEPENDENCIES,
    )?;
    unit.input_product_documents = read_id_list(
        connection,
        "SELECT product_document AS id FROM work_unit_documents WHERE work_unit = :owner ORDER BY position",
        unit.id,
        MAX_DOCUMENTS,
    )?;

    if unit.input_functional.is_some() {
        unit.input_functional = Some(read_id_list(
            connection,
            "SELECT lab_proof AS id FROM work_unit_proofs WHERE work_unit = :owner ORDER BY position",
            unit.id,
            MAX_PROOFS,
        )?);
    }

    unit.quotes = read_quotes(connection, unit.id)?;

    Ok(())
}

/// Reads the work unit `id` with its lists.
///
/// Input: `db`, the open database of the user; `id`, the id of the work unit
/// (12).
///
/// Output: the work unit; `DbError::NoRow` when there is none.
///
/// Called by: [`read_work_units`], [`check_frozen`];
/// `panels/logic/evaluation/answer.rs`: `apply_assessment`,
/// `with_dependencies`; `panels/logic/evaluation/question.rs`:
/// `assessment_question`; `panels/logic/evaluation/results.rs`:
/// `save_results`; `panels/logic/evaluation_plan/fill.rs`:
/// `apply_dependencies`, `apply_parameters`, `dependencies_question`,
/// `parameters_question`; `panels/logic/evaluation_plan/work_units.rs`:
/// `add_proof_place`, `update_work_unit_parameters`;
/// `panels/views/evaluation/run.rs`: `prepared_question`, `written`;
/// `panels/views/evaluation/work_units.rs`: `evaluation_unit_get`,
/// `needs_of`; `panels/views/evaluation_plan/fill.rs`: `prepared_question`;
/// `panels/views/evaluation_plan/work_units.rs`: `work_unit_get`.
/// Calls: [`unit_of`], [`read_lists`].
pub fn read_work_unit(db: &UserDatabase, id: i64) -> Result<WorkUnit, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM work_units WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "work_units", id });
    };
    let mut unit = unit_of(row)?;

    read_lists(connection, &mut unit)?;

    Ok(unit)
}

/// Reads every work unit of a plan with its lists, in the order of the
/// plan, one by one (five queries each).
///
/// Input: `db`, the open database of the user; `plan`, the id of the plan.
///
/// Output: the work units; an error when the plan has more than
/// [`MAX_WORK_UNITS`] (a database the program did not write).
///
/// Called by: `evaluation_plan.rs`: `create_evaluation`;
/// `panels/logic/evaluation/inputs.rs`: `give_proof`, `inputs`;
/// `panels/logic/evaluation/question.rs`: `question_of`;
/// `panels/logic/evaluation/run.rs`: `units_in_order`;
/// `panels/logic/evaluation_plan/fill.rs`: `apply_dependencies`,
/// `dependencies_question`; `panels/logic/evaluation_plan/order.rs`:
/// `check_order`; `panels/logic/evaluation_plan/plans.rs`: `update_plan`;
/// `panels/logic/evaluation_plan/work_units.rs`: `add_all_work_units`,
/// `add_work_unit`, `available_controls`;
/// `panels/views/evaluation_plan/work_units.rs`: `work_unit_list`.
/// Calls: `lists.rs`: `read_id_list`; [`read_work_unit`].
pub fn read_work_units(db: &UserDatabase, plan: i64) -> Result<Vec<WorkUnit>, DbError> {
    let ids = read_id_list(
        db.connection(),
        "SELECT id FROM work_units WHERE evaluation_plan = :owner ORDER BY position",
        plan,
        MAX_WORK_UNITS,
    )?;
    let mut units = Vec::with_capacity(ids.len());

    for id in ids {
        units.push(read_work_unit(db, id)?);
    }

    Ok(units)
}

/// The plan of the work unit `id`.
///
/// Input: `connection`, the open database; `id`, the id of the work unit.
///
/// Output: the id of its plan; `DbError::NoRow` when there is no work unit
/// `id`.
///
/// Called by: [`update_work_unit`], [`delete_work_unit`].
fn plan_of(connection: &Connection, id: i64) -> Result<i64, DbError> {
    let mut statement = connection.prepare("SELECT evaluation_plan FROM work_units WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "work_units", id });
    };

    Ok(row.get("evaluation_plan")?)
}

/// The state of the plan `plan`, as its column holds it ("plan",
/// "evaluation", "report").
///
/// Input: `connection`, the open database; `plan`, the id of the plan.
///
/// Output: the state; an error of SQLite when there is no plan `plan`.
///
/// Called by: [`check_frozen`], [`delete_work_unit`].
fn plan_state(connection: &Connection, plan: i64) -> Result<String, DbError> {
    Ok(connection.query_row(
        "SELECT state FROM evaluation_plans WHERE id = :plan",
        named_params! { ":plan": plan },
        |row| row.get(0),
    )?)
}

/// Whether two versions of a work unit have the same planning: every field
/// but its results (findings, analysis, vulnerability notes, notes, quotes,
/// verdict, author).
///
/// Input: `stored`, the work unit as stored; `unit`, the work unit to be
/// written.
///
/// Output: `true` when only the results differ.
///
/// Called by: [`check_frozen`].
fn same_planning(stored: &WorkUnit, unit: &WorkUnit) -> bool {
    stored.reference == unit.reference
        && stored.label == unit.label
        && stored.oscal_key == unit.oscal_key
        && stored.requirement == unit.requirement
        && stored.guidance == unit.guidance
        && stored.dependencies == unit.dependencies
        && stored.cross_reference == unit.cross_reference
        && stored.input_product_documents == unit.input_product_documents
        && stored.input_functional == unit.input_functional
        && stored.ai_agent == unit.ai_agent
}

/// Refuses a change of the planning of a work unit once its plan is no more
/// in planning (user, 2026-10-30): in an evaluation only its results are
/// written, so that what it was assessed against stays as it was planned.
///
/// Input: `db`, the open database of the user; `plan`, the id of its plan;
/// `unit`, the work unit to be written.
///
/// Output: nothing when its plan is in planning or only its results change;
/// otherwise `DbError::Inconsistent` ("the planning of ADV_ARC.1-1 is not
/// changed in an evaluation").
///
/// Called by: [`update_work_unit`].
/// Calls: [`plan_state`], [`read_work_unit`], [`same_planning`].
fn check_frozen(db: &UserDatabase, plan: i64, unit: &WorkUnit) -> Result<(), DbError> {
    if plan_state(db.connection(), plan)? == "plan" {
        return Ok(());
    }

    let stored = read_work_unit(db, unit.id)?;

    ensure!(
        same_planning(&stored, unit),
        DbError::Inconsistent(format!("the planning of {} is not changed in an evaluation", stored.label))
    );

    Ok(())
}

/// Writes the work unit `unit.id` anew with its lists, in one transaction;
/// it stays in its plan, at its place. In an evaluation only its results
/// may differ from what is stored.
///
/// Input: `db`, the open database of the user; `unit`, the work unit as it
/// is to be kept, its `id` that of a stored one.
///
/// Output: nothing; an error as [`check_lists`], [`check_frozen`] and
/// [`check_links`], or `DbError::NoRow` when there is no work unit
/// `unit.id`; nothing is then written.
///
/// Called by: `panels/logic/evaluation/answer.rs`: `apply_assessment`;
/// `panels/logic/evaluation/results.rs`: `save_results`;
/// `panels/logic/evaluation_plan/order.rs`: `check_order`;
/// `panels/logic/evaluation_plan/work_units.rs`: `add_proof_place`,
/// `update_work_unit_parameters`.
/// Calls: [`check_lists`], [`plan_of`], [`check_frozen`], [`check_links`],
/// [`verdict_name`], [`author_name`], [`write_lists`].
pub fn update_work_unit(db: &UserDatabase, unit: &WorkUnit) -> Result<(), DbError> {
    check_lists(unit)?;

    // The work unit stays in its plan: the plan is read from the database,
    // never taken from the caller.
    let transaction = db.connection().unchecked_transaction()?;
    let plan = plan_of(&transaction, unit.id)?;

    check_frozen(db, plan, unit)?;
    check_links(&transaction, plan, unit)?;

    transaction.execute(
        "UPDATE work_units SET reference = :reference, label = :label, oscal_key = :oscal_key,
                requirement = :requirement, guidance = :guidance, cross_reference = :cross_reference,
                functional = :functional, ai_agent = :ai_agent, findings = :findings, analysis = :analysis,
                vulnerability_notes = :vulnerability_notes, notes = :notes, verdict = :verdict, author = :author
         WHERE id = :id",
        named_params! {
            ":id": unit.id,
            ":reference": unit.reference,
            ":label": unit.label,
            ":oscal_key": unit.oscal_key,
            ":requirement": unit.requirement,
            ":guidance": unit.guidance,
            ":cross_reference": unit.cross_reference,
            ":functional": unit.input_functional.is_some(),
            ":ai_agent": unit.ai_agent,
            ":findings": unit.findings,
            ":analysis": unit.analysis,
            ":vulnerability_notes": unit.vulnerability_notes,
            ":notes": unit.notes,
            ":verdict": unit.verdict.map(verdict_name),
            ":author": unit.author.map(author_name),
        },
    )?;

    write_lists(&transaction, unit.id, unit)?;
    transaction.commit()?;

    Ok(())
}

/// Removes a work unit from its plan, with its lists and the dependencies
/// of the other work units on it (`ON DELETE CASCADE`), in one transaction:
/// the one deletion of the program, only from a plan in planning (user,
/// 2026-10-29).
///
/// Input: `db`, the open database of the user; `id`, the id of the work unit.
///
/// Output: nothing; an error when its plan is not in planning, or
/// `DbError::NoRow` when there is no work unit `id`.
///
/// Called by: `panels/logic/evaluation_plan/work_units.rs`:
/// `remove_work_unit`.
/// Calls: [`plan_of`], [`plan_state`].
pub fn delete_work_unit(db: &UserDatabase, id: i64) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let plan = plan_of(&transaction, id)?;

    ensure!(
        plan_state(&transaction, plan)? == "plan",
        DbError::Inconsistent("a work unit is removed only from a plan in planning".to_owned())
    );

    let removed = transaction.execute("DELETE FROM work_units WHERE id = :id", named_params! { ":id": id })?;

    ensure!(removed == 1, DbError::NoRow { table: "work_units", id });

    transaction.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the work units: stored with all their lists, read, changed;
    //! what does not hold together with their plan is refused.

    use super::*;

    use crate::db::models::evaluation::evaluation_plan::create_evaluation_plan;
    use crate::db::models::evaluation::lab_proof::create_lab_proof;
    use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
    use crate::model::evaluation::lab_proof::LabProof;
    use crate::test_support::database::{database, plan, unit};

    /// A proof of the laboratory, its place only reserved.
    fn proof(db: &UserDatabase) -> i64 {
        let proof =
            LabProof { id: 0, name: "Scan".to_owned(), proof: None, performed: None, tester: None, equipment: None };

        create_lab_proof(db, &proof).unwrap()
    }

    #[test]
    fn in_an_evaluation_only_the_results_of_a_work_unit_are_written() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();
        let first = create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let stored = read_work_unit(&db, first).unwrap();

        // In planning, the planning changes freely.
        update_work_unit(&db, &WorkUnit { guidance: "Look at the TOE.".to_owned(), ..stored.clone() }).unwrap();

        let evaluation = EvaluationPlan {
            state: PlanState::Evaluation,
            ..crate::db::models::evaluation::evaluation_plan::read_evaluation_plan(&db, id).unwrap()
        };

        crate::db::models::evaluation::evaluation_plan::update_evaluation_plan(&db, &evaluation).unwrap();

        let planned = read_work_unit(&db, first).unwrap();
        let assessed = WorkUnit {
            findings: "The ST names no TOE.".to_owned(),
            verdict: Some(Verdict::Fail),
            author: Some(Author::Ai),
            ..planned.clone()
        };

        update_work_unit(&db, &assessed).unwrap();
        assert_eq!(read_work_unit(&db, first).unwrap(), assessed);
        assert!(matches!(
            update_work_unit(&db, &WorkUnit { guidance: String::new(), ..assessed.clone() }),
            Err(DbError::Inconsistent(_))
        ));
        assert!(matches!(
            update_work_unit(&db, &WorkUnit { input_product_documents: Vec::new(), ..assessed }),
            Err(DbError::Inconsistent(_))
        ));
    }

    #[test]
    fn a_work_unit_is_stored_with_every_list_and_read_back() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();
        let first = create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let full = WorkUnit {
            dependencies: vec![first],
            input_functional: Some(vec![proof(&db)]),
            ai_agent: None,
            findings: "The ST names no TOE.".to_owned(),
            quotes: vec![Quote { product_document: routers.product_documents[0], text: "The TOE is".to_owned() }],
            verdict: Some(Verdict::Fail),
            author: Some(Author::Ai),
            ..unit(&routers, "ASE_INT.1-2")
        };
        let second = create_work_unit(&db, id, &full).unwrap();

        assert_eq!(read_work_unit(&db, second).unwrap(), WorkUnit { id: second, ..full });
        assert_eq!(read_work_unit(&db, first).unwrap().input_functional, None);
        assert_eq!(read_work_units(&db, id).unwrap().len(), 2);
    }

    #[test]
    fn a_work_unit_is_changed_with_its_lists() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();
        let unit_id = create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let mut stored = read_work_unit(&db, unit_id).unwrap();

        stored.verdict = Some(Verdict::Pass);
        stored.author = Some(Author::User);
        stored.input_functional = Some(Vec::new());
        update_work_unit(&db, &stored).unwrap();

        assert_eq!(read_work_unit(&db, unit_id).unwrap(), stored);
    }

    #[test]
    fn what_does_not_hold_together_with_the_plan_is_refused() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();
        let unit_id = create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let refused = [
            WorkUnit { reference: routers.oscal_documents[0] + 1, ..unit(&routers, "A") },
            WorkUnit { input_product_documents: vec![routers.product_documents[0] + 1], ..unit(&routers, "B") },
            WorkUnit { dependencies: vec![unit_id + 1], ..unit(&routers, "C") },
            WorkUnit { verdict: Some(Verdict::Inconclusive), ..unit(&routers, "D") },
            WorkUnit { quotes: vec![Quote { product_document: 99, text: "x".to_owned() }], ..unit(&routers, "E") },
            WorkUnit { id: 4, ..unit(&routers, "F") },
        ];

        for unit in &refused {
            assert!(matches!(create_work_unit(&db, id, unit), Err(DbError::Inconsistent(_))), "{}", unit.label);
        }

        let mut itself = read_work_unit(&db, unit_id).unwrap();

        itself.dependencies = vec![unit_id];

        assert!(matches!(update_work_unit(&db, &itself), Err(DbError::Inconsistent(_))));
        assert!(matches!(read_work_unit(&db, 77), Err(DbError::NoRow { .. })));
    }

    #[test]
    fn a_work_unit_is_removed_from_a_plan_with_the_dependencies_on_it() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();
        let first = create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();
        let second =
            create_work_unit(&db, id, &WorkUnit { dependencies: vec![first], ..unit(&routers, "ASE_INT.1-2") })
                .unwrap();

        delete_work_unit(&db, first).unwrap();

        assert!(matches!(read_work_unit(&db, first), Err(DbError::NoRow { .. })));
        assert!(read_work_unit(&db, second).unwrap().dependencies.is_empty());
        assert_eq!(read_work_units(&db, id).unwrap().len(), 1);
        assert!(matches!(delete_work_unit(&db, first), Err(DbError::NoRow { .. })));
    }

    #[test]
    fn a_work_unit_of_an_evaluation_is_never_removed() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();

        create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();

        let evaluation =
            crate::db::models::evaluation::evaluation_plan::create_evaluation(&db, id, "R7", "ACME", "K. K.", 1, None)
                .unwrap();
        let copied = read_work_units(&db, evaluation).unwrap();

        assert!(matches!(delete_work_unit(&db, copied[0].id), Err(DbError::Inconsistent(_))));
        assert_eq!(read_work_units(&db, evaluation).unwrap().len(), 1);
    }
}
