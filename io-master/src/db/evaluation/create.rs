//! Storing evaluations and what the evaluator adds to them.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Speaker;
use crate::model::evaluation::{
    Evaluation, Evidence, Fragment, MAX_EVALUATIONS, MAX_EVIDENCE, MAX_FRAGMENTS, MAX_MESSAGES, Package, PackageTask,
};
use crate::model::time::now;

use super::rows::{PAGE_BREAK, speaker_name};

/// A limit as a number of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Stores `evaluation` with the frozen copy of `package` (its tasks, their
/// schemes, their related elements), in one transaction; refused when the
/// package is imported already or there are [`MAX_EVALUATIONS`] already.
///
/// Called by: `package_import_import` (`views/evaluation_lab/evaluation_lab.rs`).
/// Calls: [`insert_task`].
pub fn evaluation(db: &UserDb, evaluation: &Evaluation, package: &Package) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row("SELECT COUNT(*) FROM lab_evaluations", [], |row| row.get(0))?;

    if count >= limit(MAX_EVALUATIONS)? {
        return Err(DbError::Inconsistent(format!("an evaluator keeps at most {MAX_EVALUATIONS} evaluations")));
    }

    transaction.execute(
        "INSERT INTO lab_evaluations (uuid, package_uuid, project_title, scope, package_name, evaluator, note,
                                      product, product_version, imported)
         VALUES (:uuid, :package, :title, :scope, :name, :evaluator, :note, :product, :version, :imported)",
        named_params! {
            ":uuid": evaluation.uuid,
            ":package": evaluation.package_uuid,
            ":title": evaluation.project_title,
            ":scope": evaluation.scope,
            ":name": evaluation.package_name,
            ":evaluator": evaluation.evaluator,
            ":note": evaluation.note,
            ":product": evaluation.fields.product,
            ":version": evaluation.fields.product_version,
            ":imported": now(),
        },
    )?;

    for (position, task) in package.tasks.iter().enumerate() {
        insert_task(&transaction, &evaluation.uuid, task, limit(position)?)?;
    }

    transaction.commit()?;

    Ok(())
}

/// Inserts `task` of evaluation `evaluation` at `position`, with its blocks,
/// connections and related elements, in the transaction of the caller.
///
/// Called by: [`evaluation`].
fn insert_task(
    transaction: &rusqlite::Transaction<'_>,
    evaluation: &str,
    task: &PackageTask,
    position: i64,
) -> Result<(), DbError> {
    transaction.execute(
        "INSERT INTO lab_tasks (evaluation_uuid, uuid, catalog_uuid, control_id, label, title, position, state)
         VALUES (:evaluation, :uuid, :catalog, :control, :label, :title, :position, :state)",
        named_params! {
            ":evaluation": evaluation,
            ":uuid": task.task.uuid,
            ":catalog": task.task.catalog_uuid,
            ":control": task.task.control_id,
            ":label": task.task.label,
            ":title": task.task.title,
            ":position": position,
            ":state": task.task.state.name(),
        },
    )?;

    for block in &task.blocks {
        transaction.execute(
            "INSERT INTO lab_blocks (evaluation_uuid, task_uuid, uuid, kind, x, y, name, evidence, instructions,
                                     target, format, condition)
             VALUES (:evaluation, :task, :uuid, :kind, :x, :y, :name, :evidence, :instructions, :target, :format,
                     :condition)",
            named_params! {
                ":evaluation": evaluation,
                ":task": task.task.uuid,
                ":uuid": block.uuid,
                ":kind": block.kind.name(),
                ":x": block.x,
                ":y": block.y,
                ":name": block.fields.name,
                ":evidence": block.fields.evidence,
                ":instructions": block.fields.instructions,
                ":target": block.fields.target,
                ":format": block.fields.format.name(),
                ":condition": block.fields.condition.name(),
            },
        )?;
    }

    insert_links(transaction, evaluation, task)
}

/// Inserts the connections and the related elements of `task`, in the
/// transaction of the caller.
///
/// Called by: [`insert_task`].
fn insert_links(transaction: &rusqlite::Transaction<'_>, evaluation: &str, task: &PackageTask) -> Result<(), DbError> {
    for connection in &task.connections {
        transaction.execute(
            "INSERT OR IGNORE INTO lab_connections (evaluation_uuid, task_uuid, from_block, to_block)
             VALUES (?1, ?2, ?3, ?4)",
            [evaluation, &task.task.uuid, &connection.from, &connection.to],
        )?;
    }

    for (position, related) in task.related.iter().enumerate() {
        transaction.execute(
            "INSERT INTO lab_related (evaluation_uuid, task_uuid, catalog_uuid, control_id, label, title, position)
             VALUES (:evaluation, :task, :catalog, :control, :label, :title, :position)",
            named_params! {
                ":evaluation": evaluation,
                ":task": task.task.uuid,
                ":catalog": related.catalog_uuid,
                ":control": related.control_id,
                ":label": related.label,
                ":title": related.title,
                ":position": limit(position)?,
            },
        )?;
    }

    Ok(())
}

/// Stores `evidence` of an input of task `task`, with the bytes `content`
/// exactly as delivered; refused when the input has [`MAX_EVIDENCE`]
/// already.
///
/// Called by: `evidence/add.rs`.
pub fn evidence(db: &UserDb, evaluation: &str, task: &str, evidence: &Evidence, content: &[u8]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM lab_evidence WHERE evaluation_uuid = ?1 AND block_uuid = ?2",
        [evaluation, &evidence.block],
        |row| row.get(0),
    )?;

    if count >= limit(MAX_EVIDENCE)? {
        return Err(DbError::Inconsistent(format!("an input holds at most {MAX_EVIDENCE} pieces of evidence")));
    }

    let mut pages = String::new();

    for (index, page) in evidence.pages.iter().enumerate() {
        if index > 0 {
            pages.push(PAGE_BREAK);
        }

        pages.push_str(page);
    }

    transaction.execute(
        "INSERT INTO lab_evidence (evaluation_uuid, uuid, task_uuid, block_uuid, kind, name, content, pages, added)
         VALUES (:evaluation, :uuid, :task, :block, :kind, :name, :content, :pages, :added)",
        named_params! {
            ":evaluation": evaluation,
            ":uuid": evidence.uuid,
            ":task": task,
            ":block": evidence.block,
            ":kind": evidence.kind.name(),
            ":name": evidence.name,
            ":content": content,
            ":pages": pages,
            ":added": evidence.added,
        },
    )?;
    transaction.commit()?;

    Ok(())
}

/// Adds a message of `speaker` to the chat of input `block`; refused when
/// the chat has [`MAX_MESSAGES`] already.
///
/// Called by: `input_chat/chat.rs`.
pub fn message(db: &UserDb, evaluation: &str, block: &str, speaker: Speaker, text: &str) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let (count, last): (i64, Option<i64>) = transaction.query_row(
        "SELECT COUNT(*), MAX(position) FROM lab_messages WHERE evaluation_uuid = ?1 AND block_uuid = ?2",
        [evaluation, block],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if count >= limit(MAX_MESSAGES)? {
        return Err(DbError::Inconsistent(format!("a chat holds at most {MAX_MESSAGES} messages; clear it")));
    }

    transaction.execute(
        "INSERT INTO lab_messages (evaluation_uuid, block_uuid, position, speaker, text)
         VALUES (:evaluation, :block, :position, :speaker, :text)",
        named_params! {
            ":evaluation": evaluation,
            ":block": block,
            ":position": last.unwrap_or(0) + 1,
            ":speaker": speaker_name(speaker),
            ":text": text,
        },
    )?;
    transaction.commit()?;

    Ok(())
}

/// Stores `fragment` of input `block` of task `task`; refused when the
/// input has [`MAX_FRAGMENTS`] already.
///
/// Called by: `fragments_add` (`evaluation_lab.rs`), `fragments/propose.rs`.
pub fn fragment(db: &UserDb, evaluation: &str, task: &str, block: &str, fragment: &Fragment) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM lab_fragments WHERE evaluation_uuid = ?1 AND block_uuid = ?2",
        [evaluation, block],
        |row| row.get(0),
    )?;

    if count >= limit(MAX_FRAGMENTS)? {
        return Err(DbError::Inconsistent(format!("an input holds at most {MAX_FRAGMENTS} fragments")));
    }

    transaction.execute(
        "INSERT INTO lab_fragments (evaluation_uuid, uuid, task_uuid, block_uuid, evidence_uuid, page, quote, note,
                                    found, accepted, by_ai)
         VALUES (:evaluation, :uuid, :task, :block, :evidence, :page, :quote, :note, :found, :accepted, :by_ai)",
        named_params! {
            ":evaluation": evaluation,
            ":uuid": fragment.uuid,
            ":task": task,
            ":block": block,
            ":evidence": fragment.evidence,
            ":page": fragment.page,
            ":quote": fragment.quote,
            ":note": fragment.note,
            ":found": fragment.found,
            ":accepted": fragment.accepted,
            ":by_ai": fragment.by_ai,
        },
    )?;
    transaction.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of storing evaluations.

    use super::*;

    use crate::db::evaluation::read;
    use crate::model::evaluation::{EvidenceKind, TaskState};
    use crate::test_support::db::{lab_evaluation, lab_package};
    use crate::test_support::standards::database;

    #[test]
    fn an_evaluation_keeps_the_frozen_copy_of_its_package_once() {
        let test = database();
        let package = lab_package("c");

        evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();

        assert_eq!(read::evaluation(&test.db, "e").unwrap().unwrap().fields.product, "Reader");
        assert!(read::package_imported(&test.db, "w").unwrap());
        let tasks = read::tasks(&test.db, "e").unwrap();
        assert_eq!((tasks.len(), tasks[0].state), (1, TaskState::Todo));
        assert_eq!(read::blocks(&test.db, "e", "t").unwrap().len(), 4);
        assert_eq!(read::connections(&test.db, "e", "t").unwrap().len(), 3);
        assert_eq!(read::evaluations(&test.db).unwrap()[0].tasks, 1);

        let again = Evaluation { uuid: "e2".to_owned(), ..lab_evaluation(&package) };
        assert!(evaluation(&test.db, &again, &package).is_err(), "a package is imported once");
    }

    #[test]
    fn evidence_messages_and_fragments_are_kept_for_their_input() {
        let test = database();
        let package = lab_package("c");
        evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();
        let proof = Evidence {
            uuid: "v".to_owned(),
            block: "i".to_owned(),
            kind: EvidenceKind::Pdf,
            name: "roles.pdf".to_owned(),
            pages: vec!["Page one.".to_owned(), "Page two.".to_owned()],
            added: 1,
        };

        evidence(&test.db, "e", "t", &proof, b"%PDF").unwrap();
        message(&test.db, "e", "i", Speaker::Evaluator, "Is it enough?").unwrap();
        message(&test.db, "e", "i", Speaker::Model, "Yes.").unwrap();
        let quote = Fragment {
            uuid: "f".to_owned(),
            evidence: "v".to_owned(),
            page: Some(2),
            quote: "Page two.".to_owned(),
            note: String::new(),
            found: true,
            accepted: false,
            by_ai: true,
        };
        fragment(&test.db, "e", "t", "i", &quote).unwrap();

        assert_eq!(read::evidence_of(&test.db, "e", "i").unwrap(), [proof]);
        assert_eq!(read::messages(&test.db, "e", "i").unwrap()[1].text, "Yes.");
        assert_eq!(read::fragments(&test.db, "e", "i").unwrap(), [quote]);
        assert!(read::accepted_fragments(&test.db, "e", "t").unwrap().is_empty());
    }
}
