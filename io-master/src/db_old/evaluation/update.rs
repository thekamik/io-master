//! Changing what the evaluator adds to an evaluation: the state of a task,
//! the fragments accepted, the results, the texts of the outputs.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::evaluation::{Results, TaskAgent, TaskState};

/// The name of a verdict in the database, or none.
fn verdict_name(results: &Results) -> Option<&'static str> {
    // A short closure: Option::map, which clippy asks for here.
    results.verdict.map(|verdict| verdict.name())
}

/// Sets the state of task `task`; gives whether the task exists.
///
/// Called by: `lab_tasks_state` (`views/evaluation_lab/evaluation_lab.rs`), and the
/// works on a task, which mark it in progress.
pub fn task_state(db: &UserDb, evaluation: &str, task: &str, state: TaskState) -> Result<bool, DbError> {
    let changed = db.connection().execute(
        "UPDATE lab_tasks SET state = ?1 WHERE evaluation_uuid = ?2 AND uuid = ?3",
        [state.name(), evaluation, task],
    )?;

    Ok(changed == 1)
}

/// Marks a task in progress while it is to do; one that is done stays done.
///
/// Called by: the works on a task (`evaluation_lab.rs`).
pub fn task_started(db: &UserDb, evaluation: &str, task: &str) -> Result<(), DbError> {
    db.connection().execute(
        "UPDATE lab_tasks SET state = 'working' WHERE evaluation_uuid = ?1 AND uuid = ?2 AND state = 'todo'",
        [evaluation, task],
    )?;

    Ok(())
}

/// Accepts fragment `fragment`; gives whether it was there and not
/// accepted.
///
/// Called by: `fragments_accept` (`evaluation_lab.rs`).
pub fn fragment_accepted(db: &UserDb, evaluation: &str, fragment: &str) -> Result<bool, DbError> {
    let changed = db.connection().execute(
        "UPDATE lab_fragments SET accepted = 1 WHERE evaluation_uuid = ?1 AND uuid = ?2 AND accepted = 0",
        [evaluation, fragment],
    )?;

    Ok(changed == 1)
}

/// Writes the results of task `task`, in place of the earlier ones.
///
/// Called by: `assessment_save` (`evaluation_lab.rs`).
pub fn results(db: &UserDb, evaluation: &str, task: &str, results: &Results) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT OR REPLACE INTO lab_results (evaluation_uuid, task_uuid, findings, analysis, assessment,
                                             vulnerabilities, verdict)
         VALUES (:evaluation, :task, :findings, :analysis, :assessment, :vulnerabilities, :verdict)",
        named_params! {
            ":evaluation": evaluation,
            ":task": task,
            ":findings": results.findings,
            ":analysis": results.analysis,
            ":assessment": results.assessment,
            ":vulnerabilities": results.vulnerabilities,
            ":verdict": verdict_name(results),
        },
    )?;

    Ok(())
}

/// Keeps what the AI proposed for the results of task `task`, with its
/// reason, in place of an earlier proposal.
///
/// Called by: `assessment/propose.rs`.
pub fn results_proposal(
    db: &UserDb,
    evaluation: &str,
    task: &str,
    results: &Results,
    reason: &str,
) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT OR REPLACE INTO lab_result_proposals (evaluation_uuid, task_uuid, findings, analysis, assessment,
                                                      vulnerabilities, verdict, reason)
         VALUES (:evaluation, :task, :findings, :analysis, :assessment, :vulnerabilities, :verdict, :reason)",
        named_params! {
            ":evaluation": evaluation,
            ":task": task,
            ":findings": results.findings,
            ":analysis": results.analysis,
            ":assessment": results.assessment,
            ":vulnerabilities": results.vulnerabilities,
            ":verdict": verdict_name(results),
            ":reason": reason,
        },
    )?;

    Ok(())
}

/// Writes the text of output `block` as the evaluator accepted it; the
/// proposal of the AI is decided with it.
///
/// Called by: `outputs_save`, `outputs_accept` (`evaluation_lab.rs`).
pub fn output_text(db: &UserDb, evaluation: &str, block: &str, text: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO lab_outputs (evaluation_uuid, block_uuid, text, proposal) VALUES (?1, ?2, ?3, NULL)
         ON CONFLICT (evaluation_uuid, block_uuid) DO UPDATE SET text = excluded.text, proposal = NULL",
        [evaluation, block, text],
    )?;

    Ok(())
}

/// Keeps what the AI proposed for output `block`; the accepted text stays.
///
/// Called by: `outputs/draft.rs`.
pub fn output_proposal(db: &UserDb, evaluation: &str, block: &str, proposal: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO lab_outputs (evaluation_uuid, block_uuid, text, proposal) VALUES (?1, ?2, '', ?3)
         ON CONFLICT (evaluation_uuid, block_uuid) DO UPDATE SET proposal = excluded.proposal",
        [evaluation, block, proposal],
    )?;

    Ok(())
}

/// Rejects what the AI proposed for output `block`; gives whether there was
/// a proposal.
///
/// Called by: `outputs_reject` (`evaluation_lab.rs`).
pub fn output_rejected(db: &UserDb, evaluation: &str, block: &str) -> Result<bool, DbError> {
    let changed = db.connection().execute(
        "UPDATE lab_outputs SET proposal = NULL WHERE evaluation_uuid = ?1 AND block_uuid = ?2 AND proposal IS NOT NULL",
        [evaluation, block],
    )?;

    Ok(changed == 1)
}

/// Gives evaluation `evaluation` the agent with knowledge `agent` (none:
/// `None`); the passages unticked in its tasks are forgotten, since another
/// agent finds others. One transaction.
///
/// Called by: `lab_tasks_agent` (`views/evaluation_lab/evaluation_lab.rs`).
pub fn agent(db: &UserDb, evaluation: &str, agent: Option<i64>) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM lab_agents WHERE evaluation_uuid = ?1", [evaluation])?;

    if let Some(id) = agent {
        transaction.execute(
            "INSERT INTO lab_agents (evaluation_uuid, agent_id) VALUES (:evaluation, :agent)",
            named_params! { ":evaluation": evaluation, ":agent": id },
        )?;
    }

    transaction.execute("DELETE FROM lab_context_excluded WHERE evaluation_uuid = ?1", [evaluation])?;
    transaction.commit()?;

    Ok(())
}

/// Gives task `task` the agent `agent`; its unticked passages are forgotten.
/// One transaction.
///
/// Called by: `assessment_agent` (`views/evaluation_lab/evaluation_lab.rs`).
pub fn task_agent(db: &UserDb, evaluation: &str, task: &str, agent: TaskAgent) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let keys = named_params! { ":evaluation": evaluation, ":task": task };

    transaction
        .execute("DELETE FROM lab_task_agents WHERE evaluation_uuid = :evaluation AND task_uuid = :task", keys)?;
    transaction
        .execute("DELETE FROM lab_context_excluded WHERE evaluation_uuid = :evaluation AND task_uuid = :task", keys)?;

    let stored = match agent {
        TaskAgent::AsEvaluation => None,
        TaskAgent::NoAgent => Some(None),
        TaskAgent::Agent(id) => Some(Some(id)),
    };

    if let Some(id) = stored {
        transaction.execute(
            "INSERT INTO lab_task_agents (evaluation_uuid, task_uuid, agent_id) VALUES (:evaluation, :task, :agent)",
            named_params! { ":evaluation": evaluation, ":task": task, ":agent": id },
        )?;
    }

    transaction.commit()?;

    Ok(())
}

/// Unticks (`excluded`) or ticks again passage `passage` of the agent of
/// task `task`: an unticked passage is not given to its question.
///
/// Called by: `assessment_context` (`views/evaluation_lab/evaluation_lab.rs`).
pub fn context_excluded(
    db: &UserDb,
    evaluation: &str,
    task: &str,
    passage: i64,
    excluded: bool,
) -> Result<(), DbError> {
    let keys = named_params! { ":evaluation": evaluation, ":task": task, ":passage": passage };

    if excluded {
        db.connection().execute(
            "INSERT OR IGNORE INTO lab_context_excluded (evaluation_uuid, task_uuid, passage_id)
             VALUES (:evaluation, :task, :passage)",
            keys,
        )?;
    } else {
        db.connection().execute(
            "DELETE FROM lab_context_excluded
             WHERE evaluation_uuid = :evaluation AND task_uuid = :task AND passage_id = :passage",
            keys,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of changing an evaluation.

    use super::*;

    use crate::db_old::evaluation::{create, read};
    use crate::model_old::evaluation::{Fragment, Verdict};
    use crate::test_support::db::{lab_evaluation, lab_package};
    use crate::test_support::standards::database;

    #[test]
    fn a_task_goes_in_progress_once_and_its_results_are_kept() {
        let test = database();
        let package = lab_package("c");
        create::evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();

        task_started(&test.db, "e", "t").unwrap();
        assert_eq!(read::task(&test.db, "e", "t").unwrap().unwrap().state, TaskState::Working);
        assert!(task_state(&test.db, "e", "t", TaskState::Done).unwrap());
        task_started(&test.db, "e", "t").unwrap();
        assert_eq!(read::task(&test.db, "e", "t").unwrap().unwrap().state, TaskState::Done, "done stays done");

        let mut written = Results::empty();
        written.findings = "All roles have owners.".to_owned();
        written.verdict = Some(Verdict::Met);
        results(&test.db, "e", "t", &written).unwrap();
        assert_eq!(read::results(&test.db, "e", "t").unwrap(), written);
        results_proposal(&test.db, "e", "t", &Results::empty(), "Why.").unwrap();
        assert_eq!(read::results_proposal(&test.db, "e", "t").unwrap().unwrap().1, "Why.");
    }

    #[test]
    fn a_fragment_is_accepted_once_and_an_output_keeps_its_text_beside_a_proposal() {
        let test = database();
        let package = lab_package("c");
        create::evaluation(&test.db, &lab_evaluation(&package), &package).unwrap();
        let quote = Fragment {
            uuid: "f".to_owned(),
            evidence: "v".to_owned(),
            page: None,
            quote: "q".to_owned(),
            note: String::new(),
            found: true,
            accepted: false,
            by_ai: true,
        };
        create::fragment(&test.db, "e", "t", "i", &quote).unwrap();

        assert!(fragment_accepted(&test.db, "e", "f").unwrap());
        assert!(!fragment_accepted(&test.db, "e", "f").unwrap());
        assert_eq!(read::accepted_fragments(&test.db, "e", "t").unwrap().len(), 1);

        output_text(&test.db, "e", "o", "Accepted.").unwrap();
        output_proposal(&test.db, "e", "o", "Proposed.").unwrap();
        let found = read::output(&test.db, "e", "o").unwrap().unwrap();
        assert_eq!((found.text.as_str(), found.proposal.as_deref()), ("Accepted.", Some("Proposed.")));
        assert!(output_rejected(&test.db, "e", "o").unwrap());
        assert!(!output_rejected(&test.db, "e", "o").unwrap());
    }
}
