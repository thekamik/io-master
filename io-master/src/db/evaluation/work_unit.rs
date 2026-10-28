//! The work units of the plans (`WorkUnit`), in the table `work_units` of
//! the database of the user, with their lists in tables of their own:
//! create, read, update; never deleted on their own (user, 2026-10-28): a
//! work unit leaves a plan only when the plan is written without it
//! (`update_evaluation_plan`).
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
//! each quote is taken from one of its documents.

#![allow(dead_code, reason = "the new database (2026-10-28), not used by the views yet, only by its tests")]

use rusqlite::{Connection, Row, named_params};

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::db::lists::{check_length, position, read_ids, write_ids};
use crate::model::evaluation::work_unit::{Author, Quote, Verdict, WorkUnit};

/// The most work units of one plan (the CEM has 813).
pub const MAX_WORK_UNITS: usize = 5000;

/// The most dependencies, documents, proofs and quotes of one work unit.
pub const MAX_DEPENDENCIES: usize = 100;
pub const MAX_DOCUMENTS: usize = 50;
pub const MAX_PROOFS: usize = 50;
pub const MAX_QUOTES: usize = 100;

/// The tables, made when they are missing (`database.rs`), after the
/// tables they name. `ai_agent` names an agent of `db/ai/agent.rs`; its
/// link is checked by SQLite when that table is made.
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
        ai_agent            INTEGER,
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

/// The name of a verdict in the database.
///
/// Called by: this file, `evaluation_plan.rs` (the verdicts of a plan).
pub(in crate::db) fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
        Verdict::Inconclusive => "inconclusive",
    }
}

/// The verdict of its name in the column `column` of `table`; a name the
/// program never writes is `DbError::Corrupt`.
///
/// Called by: this file, `evaluation_plan.rs` (the verdicts of a plan).
pub(in crate::db) fn verdict_of(table: &'static str, column: &'static str, name: &str) -> Result<Verdict, DbError> {
    match name {
        "pass" => Ok(Verdict::Pass),
        "fail" => Ok(Verdict::Fail),
        "inconclusive" => Ok(Verdict::Inconclusive),
        _ => Err(DbError::Corrupt { table, column, value: name.to_owned() }),
    }
}

/// The name of an author in the database.
fn author_name(author: Author) -> &'static str {
    match author {
        Author::Ai => "ai",
        Author::User => "user",
    }
}

/// The author of its name; a name the program never writes is
/// `DbError::Corrupt`.
fn author_of(name: &str) -> Result<Author, DbError> {
    match name {
        "ai" => Ok(Author::Ai),
        "user" => Ok(Author::User),
        _ => Err(DbError::Corrupt { table: "work_units", column: "author", value: name.to_owned() }),
    }
}

/// Refuses lists longer than their limits, a dependency on the work unit
/// itself, and a quote of a document the work unit does not use.
///
/// Called by: [`create_work_unit`], [`update_work_unit`].
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

/// Refuses what does not hold together with the plan `plan`: a standard
/// that is not one of the plan, a document that is not one of the plan,
/// a dependency that is no work unit of the plan, a verdict the plan does
/// not know.
///
/// Called by: [`create_work_unit`], [`update_work_unit`].
fn check_links(connection: &Connection, plan: i64, unit: &WorkUnit) -> Result<(), DbError> {
    let standards = read_ids(
        connection,
        "SELECT oscal_standard_document AS id FROM evaluation_plan_standards WHERE evaluation_plan = :owner",
        plan,
        crate::db::evaluation::evaluation_plan::MAX_STANDARDS,
    )?;
    let documents = read_ids(
        connection,
        "SELECT product_document AS id FROM evaluation_plan_documents WHERE evaluation_plan = :owner",
        plan,
        crate::db::evaluation::evaluation_plan::MAX_DOCUMENTS,
    )?;
    let units = read_ids(connection, "SELECT id FROM work_units WHERE evaluation_plan = :owner", plan, MAX_WORK_UNITS)?;

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
/// documents, its proofs and its quotes. Run inside the transaction of
/// its caller.
///
/// Called by: [`create_work_unit`], [`update_work_unit`].
/// Calls: `lists::write_ids`.
fn write_lists(connection: &Connection, id: i64, unit: &WorkUnit) -> Result<(), DbError> {
    write_ids(
        connection,
        "DELETE FROM work_unit_dependencies WHERE work_unit = :owner",
        "INSERT INTO work_unit_dependencies (work_unit, position, depends_on) VALUES (:owner, :position, :id)",
        id,
        &unit.dependencies,
    )?;
    write_ids(
        connection,
        "DELETE FROM work_unit_documents WHERE work_unit = :owner",
        "INSERT INTO work_unit_documents (work_unit, position, product_document) VALUES (:owner, :position, :id)",
        id,
        &unit.input_product_documents,
    )?;
    write_ids(
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

/// Stores a new work unit as the last of the plan `plan` and gives its id,
/// in one transaction with its lists. A new object has no id yet (`id`
/// 0): the database gives it.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_lists`], [`check_links`], [`write_lists`].
pub fn create_work_unit(db: &Database, plan: i64, unit: &WorkUnit) -> Result<i64, DbError> {
    ensure!(unit.id == 0, DbError::Inconsistent("a new work unit has an id already".to_owned()));
    check_lists(unit)?;

    let transaction = db.connection().unchecked_transaction()?;

    check_links(&transaction, plan, unit)?;

    let count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM work_units WHERE evaluation_plan = :plan",
        named_params! { ":plan": plan },
        |row| row.get(0),
    )?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_WORK_UNITS,
        DbError::Inconsistent(format!("more than {MAX_WORK_UNITS} work units in a plan"))
    );

    transaction.execute(
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

    let id = transaction.last_insert_rowid();

    write_lists(&transaction, id, unit)?;
    transaction.commit()?;

    Ok(id)
}

/// The model of a row of `work_units`, its lists still empty (`read_lists`
/// fills them); `functional` gives `input_functional` an empty list or
/// `None`.
///
/// Called by: [`read_work_unit`].
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

/// Fills the lists of `unit` from their tables.
///
/// Called by: [`read_work_unit`].
/// Calls: `lists::read_ids`, [`read_quotes`].
fn read_lists(connection: &Connection, unit: &mut WorkUnit) -> Result<(), DbError> {
    unit.dependencies = read_ids(
        connection,
        "SELECT depends_on AS id FROM work_unit_dependencies WHERE work_unit = :owner ORDER BY position",
        unit.id,
        MAX_DEPENDENCIES,
    )?;
    unit.input_product_documents = read_ids(
        connection,
        "SELECT product_document AS id FROM work_unit_documents WHERE work_unit = :owner ORDER BY position",
        unit.id,
        MAX_DOCUMENTS,
    )?;

    if unit.input_functional.is_some() {
        unit.input_functional = Some(read_ids(
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
/// Called by: [`read_work_units`]; the views rewritten on `db/`.
/// Calls: [`unit_of`], [`read_lists`].
pub fn read_work_unit(db: &Database, id: i64) -> Result<WorkUnit, DbError> {
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

/// Reads every work unit of the plan `plan`, in the order of the plan.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: `lists::read_ids`, [`read_work_unit`].
pub fn read_work_units(db: &Database, plan: i64) -> Result<Vec<WorkUnit>, DbError> {
    let ids = read_ids(
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
/// Called by: [`update_work_unit`].
fn plan_of(connection: &Connection, id: i64) -> Result<i64, DbError> {
    let mut statement = connection.prepare("SELECT evaluation_plan FROM work_units WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "work_units", id });
    };

    Ok(row.get("evaluation_plan")?)
}

/// Writes the work unit `unit.id` anew, with its lists, in one
/// transaction; it stays in its plan, at its place.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_lists`], [`plan_of`], [`check_links`], [`write_lists`].
pub fn update_work_unit(db: &Database, unit: &WorkUnit) -> Result<(), DbError> {
    check_lists(unit)?;

    let transaction = db.connection().unchecked_transaction()?;
    let plan = plan_of(&transaction, unit.id)?;

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

#[cfg(test)]
mod tests {
    //! Tests of the work units: stored with all their lists, read, changed;
    //! what does not hold together with their plan is refused.

    use super::*;

    use crate::db::evaluation::evaluation_plan::create_evaluation_plan;
    use crate::db::evaluation::lab_proof::create_lab_proof;
    use crate::model::evaluation::lab_proof::LabProof;
    use crate::test_support::database::{database, plan, unit};

    /// A proof of the laboratory, its place only reserved.
    fn proof(db: &Database) -> i64 {
        let proof =
            LabProof { id: 0, name: "Scan".to_owned(), proof: None, performed: None, tester: None, equipment: None };

        create_lab_proof(db, &proof).unwrap()
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
}
