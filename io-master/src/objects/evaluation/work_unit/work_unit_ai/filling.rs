//! "Fill with AI" on one work unit of a plan in planning (user,
//! 2026-10-29), as a step of the filling of the plan
//! (`evaluation_plan_ai.rs`: `fill_plan`): the question of its task, its
//! answer read and written as the planner would write it (`proposal.rs`).
//!
//! ```text
//! fill_work_unit ──► prepared_question  (the lock of the database held)  parameters_question / dependencies_question
//!                ──► send_whole         (released)                        the answer of the AI
//!                ──► written            (held)                            record (the journal of the agent),
//!                                        applied: read_* and apply_* ──► filled, the notes
//! ```
//!
//! A work unit the AI cannot fill (a question refused, an answer that is no
//! proposal) is left as it was, with the reason; only a failure of the
//! session or of the database is an error.

use std::sync::Mutex;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::integration::integration_ai::question::{Prepared, prepare};
use crate::objects::ai::integration::integration_ai::sending::{Sent, send_whole};
use crate::objects::ai::journal::journal_ai::record;
use crate::objects::ai_error::AiError;
use crate::objects::evaluation::work_unit::work_unit_ai::proposal::{
    apply_dependencies, apply_parameters, dependencies_question, parameters_question, read_dependencies,
    read_parameters,
};
use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
use crate::objects::session::progress::progress_ai::lock::{lock_database, open_database};

/// The task of a filling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillTask {
    /// Every parameter but the dependencies.
    Parameters,
    /// The dependencies only, when the work units are described.
    Dependencies,
}

/// The label of the work unit `unit` and the question of the task, prepared,
/// under the lock of the database.
///
/// Input: `database`, the lock of the open database of the session; `task`,
/// the task of the filling; `plan`, a plan in planning with an agent of the
/// planner; `unit`, one of its work units.
/// Output: the label and the question (`("1.1.1", Ok(prepared))`), or the
/// reason instead of the question when it is refused (no room in the
/// context, no key); an error when the session or the database fails.
///
/// Called by: [`fill_work_unit`].
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_database`, `open_database`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`;
/// `objects/evaluation/work_unit/work_unit_ai/proposal.rs`: `parameters_question`, `dependencies_question`;
/// `objects/ai/integration/integration_ai/question.rs`: `prepare`.
fn prepared_question(
    database: &Mutex<Option<UserDatabase>>,
    task: FillTask,
    plan: i64,
    unit: i64,
) -> Result<(String, Result<Prepared, String>), AiError> {
    let opened = lock_database(database)?;
    let db = open_database(opened.as_ref())?;
    let label = read_work_unit(db, unit)?.label;
    let question = match task {
        FillTask::Parameters => parameters_question(db, plan, unit),
        FillTask::Dependencies => dependencies_question(db, plan, unit),
    };
    let prepared = match question {
        Ok(question) => prepare(db, &question).map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };

    Ok((label, prepared))
}

/// Reads the answer `answer` as the task asks and writes it into the work
/// unit `unit`.
///
/// Input: `task`, the task of the filling; `plan`, `unit`, as
/// [`prepared_question`]; `answer`, the text of the answer of the AI.
/// Output: the notes of what was left out (`["the proof X is unknown"]`);
/// `AiError::Refused` for an answer that cannot be read, an error of the
/// database.
///
/// Called by: [`written`].
/// Calls: `objects/evaluation/work_unit/work_unit_ai/proposal.rs`: `read_parameters`, `apply_parameters`,
/// `read_dependencies`, `apply_dependencies`.
fn applied(db: &UserDatabase, task: FillTask, plan: i64, unit: i64, answer: &str) -> Result<Vec<String>, AiError> {
    match task {
        FillTask::Parameters => apply_parameters(db, plan, unit, &read_parameters(answer)?),
        FillTask::Dependencies => apply_dependencies(db, plan, unit, &read_dependencies(answer)?),
    }
}

/// Writes the answer into the journal of the agent of the planner, then
/// into the work unit, under the lock of the database.
///
/// Input: `database`, the lock of the open database of the session; `step`,
/// the task and the ids of the plan and of the work unit; `prepared`, the
/// question as it was sent; `sent`, its answer.
/// Output: whether it was filled and the notes (`(true, [])`); an error when
/// the session or the database fails.
///
/// Called by: [`fill_work_unit`].
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_database`, `open_database`;
/// `objects/ai/journal/journal_ai.rs`: `record`; [`applied`].
fn written(
    database: &Mutex<Option<UserDatabase>>,
    step: (FillTask, i64, i64),
    prepared: Prepared,
    sent: Sent,
) -> Result<(bool, Vec<String>), AiError> {
    let (task, plan, unit) = step;
    let opened = lock_database(database)?;
    let db = open_database(opened.as_ref())?;
    let answer = record(db, prepared, sent)?;

    if let Some(error) = answer.error {
        return Ok((false, vec![error.to_string()]));
    }

    Ok(match applied(db, task, plan, unit, &answer.text) {
        Ok(notes) => (true, notes),
        Err(error) => (false, vec![error.to_string()]),
    })
}

/// Fills the work unit `unit` of the plan `plan` with the task `task`: the
/// question prepared under the lock of the database, sent without it, the
/// answer written under it again.
///
/// Input:
/// - `database`: the lock of the open database of the session.
/// - `task`: the parameters, or the dependencies.
/// - `plan`: a plan in planning with an agent of the planner; `unit`: one of
///   its work units.
///
/// Output: the label of the work unit, whether it was filled, and the
/// notes: what was left out of the answer, or why it was not filled
/// (`("1.1.1", true, [])`). An error when the session or the database fails.
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `fill_plan`.
/// Calls: [`prepared_question`], `objects/ai/integration/integration_ai/sending.rs`:
/// `send_whole`, [`written`].
pub async fn fill_work_unit(
    database: &Mutex<Option<UserDatabase>>,
    task: FillTask,
    plan: i64,
    unit: i64,
) -> Result<(String, bool, Vec<String>), AiError> {
    let (label, prepared) = prepared_question(database, task, plan, unit)?;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return Ok((label, false, vec![reason])),
    };
    let sent = send_whole(&prepared).await;
    let (filled, notes) = written(database, (task, plan, unit), prepared, sent)?;

    Ok((label, filled, notes))
}
