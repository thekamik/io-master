//! Reading evaluations and what the evaluator added to them.

use rusqlite::OptionalExtension;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::ai::Turn;
use crate::model_old::evaluation::{
    Evaluation, EvaluationSummary, Evidence, Fragment, LabTask, MAX_EVALUATIONS, MAX_EVIDENCE, MAX_FRAGMENTS,
    MAX_MESSAGES, OutputText, Related, Results, TaskAgent, TaskProgress,
};
use crate::model_old::evaluation_project::{MAX_LINKS, MAX_TASKS};
use crate::model_old::knowledge::MAX_HITS;
use crate::model_old::task_scheme::{Block, Connection, MAX_BLOCKS};

use super::rows;

/// A limit as a parameter of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// The evaluations with the numbers of their tasks and of the tasks done,
/// the newest import first; at most `MAX_EVALUATIONS`.
///
/// Called by: `evaluation_list_get` (`views/evaluation_lab/evaluation_lab.rs`).
pub fn evaluations(db: &UserDb) -> Result<Vec<EvaluationSummary>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT e.uuid AS uuid, e.project_title AS project_title, e.package_name AS package_name,
                e.product AS product,
                (SELECT COUNT(*) FROM lab_tasks t WHERE t.evaluation_uuid = e.uuid) AS tasks,
                (SELECT COUNT(*) FROM lab_tasks t WHERE t.evaluation_uuid = e.uuid AND t.state = 'done') AS done
         FROM lab_evaluations e ORDER BY e.imported DESC, e.rowid DESC LIMIT ?1",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map([limit(MAX_EVALUATIONS)?], |row| Ok(rows::summary(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// Evaluation `uuid`; `None` when it does not exist.
///
/// Called by: the commands of an evaluation (`evaluation_lab.rs`).
pub fn evaluation(db: &UserDb, uuid: &str) -> Result<Option<Evaluation>, DbError> {
    db.connection()
        .query_row("SELECT * FROM lab_evaluations WHERE uuid = ?1", [uuid], |row| Ok(rows::evaluation(row)))
        .optional()?
        .transpose()
}

/// Whether package `package` is imported already.
///
/// Called by: `package_import_import` (`evaluation_lab.rs`).
pub fn package_imported(db: &UserDb, package: &str) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT COUNT(*) > 0 FROM lab_evaluations WHERE package_uuid = ?1",
        [package],
        |row| row.get(0),
    )?)
}

/// The tasks of evaluation `evaluation`, in their order; at most `MAX_TASKS`.
///
/// Called by: `lab_tasks_get` (`evaluation_lab.rs`), `report/export.rs`.
pub fn tasks(db: &UserDb, evaluation: &str) -> Result<Vec<LabTask>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM lab_tasks WHERE evaluation_uuid = ?1 ORDER BY position LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![evaluation, limit(MAX_TASKS)?], |row| Ok(rows::task(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// Task `task` of evaluation `evaluation`; `None` when it does not exist.
///
/// Called by: the commands of a task (`evaluation_lab.rs`).
pub fn task(db: &UserDb, evaluation: &str, task: &str) -> Result<Option<LabTask>, DbError> {
    db.connection()
        .query_row("SELECT * FROM lab_tasks WHERE evaluation_uuid = ?1 AND uuid = ?2", [evaluation, task], |row| {
            Ok(rows::task(row))
        })
        .optional()?
        .transpose()
}

/// The blocks of the scheme of task `task`: its requirement first, then in
/// the order of the package; at most `MAX_BLOCKS`.
///
/// Called by: `lab_task/read.rs` and the works of the AI of a task.
pub fn blocks(db: &UserDb, evaluation: &str, task: &str) -> Result<Vec<Block>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT * FROM lab_blocks WHERE evaluation_uuid = ?1 AND task_uuid = ?2
         ORDER BY kind = 'requirement' DESC, rowid LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in
        statement.query_map(rusqlite::params![evaluation, task, limit(MAX_BLOCKS)?], |row| Ok(rows::block(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// Block `block` of evaluation `evaluation` and its task; `None` when it does
/// not exist.
///
/// Called by: the commands of an input or an output (`evaluation_lab.rs`).
pub fn block(db: &UserDb, evaluation: &str, block: &str) -> Result<Option<(String, Block)>, DbError> {
    let found = db
        .connection()
        .query_row("SELECT * FROM lab_blocks WHERE evaluation_uuid = ?1 AND uuid = ?2", [evaluation, block], |row| {
            Ok((row.get::<_, String>("task_uuid"), rows::block(row)))
        })
        .optional()?;

    match found {
        Some((task, block)) => Ok(Some((task?, block?))),
        None => Ok(None),
    }
}

/// The connections of the blocks of task `task`; at most twice `MAX_BLOCKS`.
///
/// Called by: `lab_task/read.rs`.
pub fn connections(db: &UserDb, evaluation: &str, task: &str) -> Result<Vec<Connection>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT * FROM lab_connections WHERE evaluation_uuid = ?1 AND task_uuid = ?2 ORDER BY rowid LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in statement
        .query_map(rusqlite::params![evaluation, task, limit(MAX_BLOCKS * 2)?], |row| Ok(rows::connection(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// The related elements of task `task`, in their order; at most `MAX_LINKS`.
///
/// Called by: `lab_task/read.rs`.
pub fn related(db: &UserDb, evaluation: &str, task: &str) -> Result<Vec<Related>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT * FROM lab_related WHERE evaluation_uuid = ?1 AND task_uuid = ?2 ORDER BY position LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in
        statement.query_map(rusqlite::params![evaluation, task, limit(MAX_LINKS)?], |row| Ok(rows::related(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// The evidence of input `block`, in the order delivered, without its bytes;
/// at most `MAX_EVIDENCE`.
///
/// Called by: `lab_task/read.rs`, the chat and the fragments of an input.
pub fn evidence_of(db: &UserDb, evaluation: &str, block: &str) -> Result<Vec<Evidence>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT uuid, block_uuid, kind, name, pages, added FROM lab_evidence
         WHERE evaluation_uuid = ?1 AND block_uuid = ?2 ORDER BY added, rowid LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in statement
        .query_map(rusqlite::params![evaluation, block, limit(MAX_EVIDENCE)?], |row| Ok(rows::evidence(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// The chat of input `block`, in order; at most `MAX_MESSAGES`.
///
/// Called by: `lab_task/read.rs`, `input_chat/chat.rs`.
pub fn messages(db: &UserDb, evaluation: &str, block: &str) -> Result<Vec<Turn>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT speaker, text FROM lab_messages WHERE evaluation_uuid = ?1 AND block_uuid = ?2
         ORDER BY position LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in
        statement.query_map(rusqlite::params![evaluation, block, limit(MAX_MESSAGES)?], |row| Ok(rows::message(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// The fragments of input `block`, in the order found; at most
/// `MAX_FRAGMENTS`.
///
/// Called by: `lab_task/read.rs`.
pub fn fragments(db: &UserDb, evaluation: &str, block: &str) -> Result<Vec<Fragment>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT * FROM lab_fragments WHERE evaluation_uuid = ?1 AND block_uuid = ?2 ORDER BY rowid LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in statement
        .query_map(rusqlite::params![evaluation, block, limit(MAX_FRAGMENTS)?], |row| Ok(rows::fragment(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// The accepted fragments of the inputs of task `task`, each with the UUID
/// of its input; at most `MAX_FRAGMENTS` for each block.
///
/// Called by: `assessment/propose.rs`.
pub fn accepted_fragments(db: &UserDb, evaluation: &str, task: &str) -> Result<Vec<(String, Fragment)>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT * FROM lab_fragments WHERE evaluation_uuid = ?1 AND task_uuid = ?2 AND accepted = 1
         ORDER BY block_uuid, rowid LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![evaluation, task, limit(MAX_FRAGMENTS * MAX_BLOCKS)?], |row| {
        Ok((row.get::<_, String>("block_uuid"), rows::fragment(row)))
    })? {
        let (block, fragment) = row?;
        found.push((block?, fragment?));
    }

    Ok(found)
}

/// The four results and the verdict of task `task`; empty while none is
/// written.
///
/// Called by: `lab_task/read.rs`, `outputs/draft.rs`, `report/export.rs`.
pub fn results(db: &UserDb, evaluation: &str, task: &str) -> Result<Results, DbError> {
    let found = db
        .connection()
        .query_row(
            "SELECT * FROM lab_results WHERE evaluation_uuid = ?1 AND task_uuid = ?2",
            [evaluation, task],
            |row| Ok(rows::results(row, "lab_results")),
        )
        .optional()?
        .transpose()?;

    Ok(found.unwrap_or_else(Results::empty))
}

/// What the AI proposed for the results of task `task`, with its reason;
/// `None` when nothing is waiting.
///
/// Called by: `lab_task/read.rs`, `assessment_accept` (`evaluation_lab.rs`).
pub fn results_proposal(db: &UserDb, evaluation: &str, task: &str) -> Result<Option<(Results, String)>, DbError> {
    let found = db
        .connection()
        .query_row(
            "SELECT * FROM lab_result_proposals WHERE evaluation_uuid = ?1 AND task_uuid = ?2",
            [evaluation, task],
            |row| Ok((rows::results(row, "lab_result_proposals"), row.get::<_, String>("reason"))),
        )
        .optional()?;

    match found {
        Some((results, reason)) => Ok(Some((results?, reason?))),
        None => Ok(None),
    }
}

/// The text of output `block`; `None` while nothing is written.
///
/// Called by: `lab_task/read.rs`, `outputs_accept` (`evaluation_lab.rs`), `report/export.rs`.
pub fn output(db: &UserDb, evaluation: &str, block: &str) -> Result<Option<OutputText>, DbError> {
    db.connection()
        .query_row(
            "SELECT * FROM lab_outputs WHERE evaluation_uuid = ?1 AND block_uuid = ?2",
            [evaluation, block],
            |row| Ok(rows::output(row)),
        )
        .optional()?
        .transpose()
}

/// The agent with knowledge of evaluation `evaluation`; `None` when it has none.
///
/// Called by: `views/evaluation_lab/assessment/context.rs`, `lab_tasks/view.rs`.
pub fn agent(db: &UserDb, evaluation: &str) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT agent_id FROM lab_agents WHERE evaluation_uuid = ?1", [evaluation], |row| row.get(0))
        .optional()?)
}

/// The agent of task `task`: that of its evaluation when no other was chosen.
///
/// Called by: `views/evaluation_lab/assessment/context.rs`.
pub fn task_agent(db: &UserDb, evaluation: &str, task: &str) -> Result<TaskAgent, DbError> {
    let stored: Option<Option<i64>> = db
        .connection()
        .query_row(
            "SELECT agent_id FROM lab_task_agents WHERE evaluation_uuid = ?1 AND task_uuid = ?2",
            [evaluation, task],
            |row| row.get(0),
        )
        .optional()?;

    Ok(match stored {
        None => TaskAgent::AsEvaluation,
        Some(None) => TaskAgent::NoAgent,
        Some(Some(id)) => TaskAgent::Agent(id),
    })
}

/// The passages the evaluator unticked for task `task`; at most `MAX_HITS`,
/// the most an agent ever shows.
///
/// Called by: `views/evaluation_lab/assessment/context.rs`.
pub fn context_excluded(db: &UserDb, evaluation: &str, task: &str) -> Result<Vec<i64>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT passage_id FROM lab_context_excluded WHERE evaluation_uuid = ?1 AND task_uuid = ?2 LIMIT ?3",
    )?;
    let mut found = Vec::new();

    for passage in statement.query_map(rusqlite::params![evaluation, task, limit(MAX_HITS)?], |row| row.get(0))? {
        found.push(passage?);
    }

    Ok(found)
}

/// How far every task of evaluation `evaluation` is, in the order of the
/// package: its verdict, its accepted fragments, its outputs with their
/// condition and whether their text is accepted (step 59).
///
/// Called by: `views/evaluation_lab/lab_tasks/view.rs` (the screen of an evaluation).
pub fn progress(db: &UserDb, evaluation: &str) -> Result<Vec<TaskProgress>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT t.uuid AS task,
                (SELECT r.verdict FROM lab_results r
                  WHERE r.evaluation_uuid = t.evaluation_uuid AND r.task_uuid = t.uuid) AS verdict,
                (SELECT COUNT(*) FROM lab_fragments f
                  WHERE f.evaluation_uuid = t.evaluation_uuid AND f.task_uuid = t.uuid AND f.accepted = 1) AS fragments
         FROM lab_tasks t WHERE t.evaluation_uuid = ?1 ORDER BY t.position LIMIT ?2",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![evaluation, limit(MAX_TASKS)?], |row| Ok(rows::progress(row)))? {
        found.push(row??);
    }

    let mut statement = connection.prepare(
        "SELECT b.task_uuid AS task, b.condition AS condition, COALESCE(length(o.text), 0) > 0 AS ready
         FROM lab_blocks b LEFT JOIN lab_outputs o ON o.evaluation_uuid = b.evaluation_uuid AND o.block_uuid = b.uuid
         WHERE b.evaluation_uuid = ?1 AND b.kind = 'output' LIMIT ?2",
    )?;
    let max_outputs = limit(MAX_TASKS.saturating_mul(MAX_BLOCKS))?;

    for row in statement.query_map(rusqlite::params![evaluation, max_outputs], |row| Ok(rows::output_state(row)))? {
        let (task, condition, ready) = row??;

        for progress in &mut found {
            if progress.task == task {
                progress.outputs.push((condition, ready));
            }
        }
    }

    Ok(found)
}
