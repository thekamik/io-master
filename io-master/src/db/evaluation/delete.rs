//! Deleting evaluations and what the evaluator added to them.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// The tables that hold the rows of an evaluation, the evaluation itself last.
const TABLES: [&str; 14] = [
    "lab_context_excluded",
    "lab_task_agents",
    "lab_agents",
    "lab_outputs",
    "lab_result_proposals",
    "lab_results",
    "lab_fragments",
    "lab_messages",
    "lab_evidence",
    "lab_related",
    "lab_connections",
    "lab_blocks",
    "lab_tasks",
    "lab_evaluations",
];

/// Deletes evaluation `uuid` with everything of it, in one transaction;
/// gives whether it existed.
///
/// Called by: `evaluation_list_delete` (`views/evaluation_lab/evaluation_lab.rs`).
pub fn evaluation(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let mut deleted = false;

    for table in TABLES {
        let column = if table == "lab_evaluations" { "uuid" } else { "evaluation_uuid" };
        // The names come from the constant above, never from outside.
        let changed = transaction.execute(&format!("DELETE FROM {table} WHERE {column} = ?1"), [uuid])?;

        deleted = changed > 0 && table == "lab_evaluations";
    }

    transaction.commit()?;

    Ok(deleted)
}

/// Deletes piece of evidence `uuid` with its fragments, in one
/// transaction; gives whether it existed.
///
/// Called by: `evidence_delete` (`evaluation_lab.rs`).
pub fn evidence(db: &UserDb, evaluation: &str, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction
        .execute("DELETE FROM lab_fragments WHERE evaluation_uuid = ?1 AND evidence_uuid = ?2", [evaluation, uuid])?;
    let deleted = transaction
        .execute("DELETE FROM lab_evidence WHERE evaluation_uuid = ?1 AND uuid = ?2", [evaluation, uuid])?
        > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes fragment `uuid` (rejected, or removed by the evaluator); gives
/// whether it existed.
///
/// Called by: `fragments_remove` (`evaluation_lab.rs`).
pub fn fragment(db: &UserDb, evaluation: &str, uuid: &str) -> Result<bool, DbError> {
    Ok(db
        .connection()
        .execute("DELETE FROM lab_fragments WHERE evaluation_uuid = ?1 AND uuid = ?2", [evaluation, uuid])?
        > 0)
}

/// Rejects the fragments of input `block` the AI proposed and the evaluator
/// did not accept; run before a new question.
///
/// Called by: `fragments/propose.rs`.
pub fn proposed_fragments(db: &UserDb, evaluation: &str, block: &str) -> Result<(), DbError> {
    db.connection().execute(
        "DELETE FROM lab_fragments WHERE evaluation_uuid = ?1 AND block_uuid = ?2 AND by_ai = 1 AND accepted = 0",
        [evaluation, block],
    )?;

    Ok(())
}

/// Clears the chat of input `block`.
///
/// Called by: `input_chat_clear` (`evaluation_lab.rs`).
pub fn messages(db: &UserDb, evaluation: &str, block: &str) -> Result<(), DbError> {
    db.connection()
        .execute("DELETE FROM lab_messages WHERE evaluation_uuid = ?1 AND block_uuid = ?2", [evaluation, block])?;

    Ok(())
}

/// Removes what the AI proposed for the results of task `task` (accepted
/// or rejected); gives whether there was a proposal.
///
/// Called by: `assessment_accept`, `assessment_reject` (`evaluation_lab.rs`).
pub fn results_proposal(db: &UserDb, evaluation: &str, task: &str) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "DELETE FROM lab_result_proposals WHERE evaluation_uuid = ?1 AND task_uuid = ?2",
        [evaluation, task],
    )? > 0)
}

#[cfg(test)]
mod tests {
    //! Tests of deleting an evaluation and its parts.

    use super::*;

    use crate::db::evaluation::{create, read};
    use crate::model::ai::Speaker;
    use crate::model::evaluation::{Evidence, EvidenceKind, Fragment};
    use crate::test_support::db::{lab_evaluation, lab_package};
    use crate::test_support::standards::database;

    #[test]
    fn evidence_goes_with_its_fragments_and_an_evaluation_with_everything() {
        let test = database();
        let package = lab_package("c");
        create::evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();
        let proof = Evidence {
            uuid: "v".to_owned(),
            block: "i".to_owned(),
            kind: EvidenceKind::Note,
            name: "Interview".to_owned(),
            pages: vec!["Notes.".to_owned()],
            added: 1,
        };
        create::evidence(&test.db, "e", "t", &proof, b"Notes.").unwrap();
        for (uuid, by_ai) in [("f1", true), ("f2", false)] {
            let quote = Fragment {
                uuid: uuid.to_owned(),
                evidence: "v".to_owned(),
                page: None,
                quote: "Notes.".to_owned(),
                note: String::new(),
                found: true,
                accepted: false,
                by_ai,
            };
            create::fragment(&test.db, "e", "t", "i", &quote).unwrap();
        }
        create::message(&test.db, "e", "i", Speaker::Evaluator, "Hello").unwrap();

        proposed_fragments(&test.db, "e", "i").unwrap();
        assert_eq!(read::fragments(&test.db, "e", "i").unwrap().len(), 1, "the evaluator's own stays");
        messages(&test.db, "e", "i").unwrap();
        assert!(read::messages(&test.db, "e", "i").unwrap().is_empty());
        assert!(evidence(&test.db, "e", "v").unwrap());
        assert!(read::fragments(&test.db, "e", "i").unwrap().is_empty());

        assert!(evaluation(&test.db, "e").unwrap());
        assert!(!evaluation(&test.db, "e").unwrap());
        assert!(read::tasks(&test.db, "e").unwrap().is_empty());
        assert!(!read::package_imported(&test.db, "w").unwrap());
    }
}
