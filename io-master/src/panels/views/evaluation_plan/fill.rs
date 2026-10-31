//! "Fill with AI", as the page calls it (user, 2026-10-29): the agent of
//! the planner fills every work unit of the plan, one after the other; two
//! commands, two tasks, each started by the planner on its own: the
//! parameters, then the dependencies (when the work units are described).
//! The page sees each step as it ends and may stop the filling.
//!
//! ```text
//! PlanFill.svelte ──plan_fill_preview(plan)──► fill_preview: the questions counted now ──► FillPreviewView
//!                 ──plan_fill_parameters(plan, on_step)───┐
//!                 ──plan_fill_dependencies(plan, on_step)─┴► fill: fill_units, then for each work unit:
//!                     (the lock of the session held)  the question of the task, ask::prepare
//!                     (released)                      answer::send_drained
//!                     (held)                          ask::record (the journal of the agent),
//!                                                     the answer read and written by the task
//!                     ──on_step──► FillStepView (the work unit, filled or not, the notes)
//!                 ──plan_fill_stop()──► the session asked to stop: the filling ends after its present step
//! ```
//!
//! Only one filling runs at a time (`Session::begin_work`). A work unit the
//! AI could not fill is told and passed over; a failure of the session or
//! of the database ends the filling.

use serde::Serialize;
use tauri::State;
use tauri::ipc::Channel;

use crate::ai::ask::{Prepared, Sent, prepare, record};
use crate::db::models::evaluation::work_unit::read_work_unit;
use crate::db::storage::user_database::UserDatabase;
use crate::panels::logic::evaluation_plan::error::PlanError;
use crate::panels::logic::evaluation_plan::fill::{
    apply_dependencies, apply_parameters, dependencies_question, fill_preview, fill_units, parameters_question,
    read_dependencies, read_parameters,
};
use crate::panels::views::answer::send_drained;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::session::state::Session;

/// One step of a filling, as the page shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillStepView {
    /// The place of the work unit in the plan, from 1, and how many there
    /// are.
    number: usize,
    total: usize,
    label: String,
    /// The work unit was filled; otherwise `notes` tell why it was not.
    filled: bool,
    /// What was left out of the answer, or why the work unit was not filled.
    notes: Vec<String>,
}

/// What a filling would ask, counted when the planner asks it.
#[derive(Serialize)]
pub struct FillPreviewView {
    questions: usize,
    agent: String,
    integration: String,
}

/// How a filling ended.
#[derive(Serialize)]
pub struct FillSummaryView {
    filled: usize,
    failed: usize,
    /// The user stopped it before its end.
    stopped: bool,
}

/// The task of a filling.
#[derive(Clone, Copy)]
enum Task {
    /// Every parameter but the dependencies.
    Parameters,
    /// The dependencies only.
    Dependencies,
}

/// What became of one work unit.
enum Outcome {
    /// Filled; what was left out of the answer.
    Filled(Vec<String>),
    /// Not filled; why.
    Failed(String),
}

/// The label of the work unit `unit` and the question of the task, prepared;
/// the reason when the question is refused (no room in the context, no key).
///
/// Called by: [`fill_step`].
/// Calls: `read_work_unit`, `parameters_question`, `dependencies_question`,
/// `ask::prepare`.
fn prepared_question(
    session: &Session,
    task: Task,
    plan: i64,
    unit: i64,
) -> Result<(String, Result<Prepared, String>), ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let label = read_work_unit(db, unit)?.label;
    let question = match task {
        Task::Parameters => parameters_question(db, plan, unit),
        Task::Dependencies => dependencies_question(db, plan, unit),
    };
    let prepared = match question {
        Ok(question) => prepare(db, &question).map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };

    Ok((label, prepared))
}

/// Reads the answer `answer` as the task asks and writes it into the work
/// unit `unit`; gives the notes of what was left out.
///
/// Called by: [`written`].
/// Calls: `read_parameters`, `apply_parameters`, `read_dependencies`,
/// `apply_dependencies`.
fn applied(db: &UserDatabase, task: Task, plan: i64, unit: i64, answer: &str) -> Result<Vec<String>, PlanError> {
    match task {
        Task::Parameters => apply_parameters(db, plan, unit, &read_parameters(answer)?),
        Task::Dependencies => apply_dependencies(db, plan, unit, &read_dependencies(answer)?),
    }
}

/// Writes the answer into the journal of the agent of the planner, then
/// into the work unit `unit`.
///
/// Called by: [`fill_step`].
/// Calls: `ask::record`, [`applied`].
fn written(session: &Session, step: (Task, i64, i64), prepared: Prepared, sent: Sent) -> Result<Outcome, ViewError> {
    let (task, plan, unit) = step;
    let opened = session.lock()?;
    let db = database(&opened)?;
    let answer = record(db, prepared, sent)?;

    if let Some(error) = answer.error {
        return Ok(Outcome::Failed(error.to_string()));
    }

    Ok(match applied(db, task, plan, unit, &answer.text) {
        Ok(notes) => Outcome::Filled(notes),
        Err(error) => Outcome::Failed(error.to_string()),
    })
}

/// Fills the work unit `unit` with the task `task`: the question prepared
/// under the lock of the session, sent without it, the answer written under
/// it again.
///
/// Called by: [`fill`].
/// Calls: [`prepared_question`], `answer::send_drained`, [`written`].
async fn fill_step(session: &Session, task: Task, plan: i64, unit: i64) -> Result<(String, Outcome), ViewError> {
    let (label, prepared) = prepared_question(session, task, plan, unit)?;
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => return Ok((label, Outcome::Failed(reason))),
    };
    let sent = send_drained(&prepared).await;

    Ok((label, written(session, (task, plan, unit), prepared, sent)?))
}

/// Fills every work unit of the plan `plan` in its order with the task
/// `task`, telling the page each step through `on_step`. Refused while
/// another filling runs, or when the plan has no agent of the planner.
///
/// Called by: [`plan_fill_parameters`], [`plan_fill_dependencies`].
/// Calls: `Session::begin_work`, `fill_units`, [`fill_step`].
async fn fill(
    session: &Session,
    task: Task,
    plan: i64,
    on_step: &Channel<FillStepView>,
) -> Result<FillSummaryView, ViewError> {
    let Some(_work) = session.begin_work() else {
        return Err(ViewError::Invalid("a filling runs already".to_owned()));
    };
    let units = {
        let opened = session.lock()?;

        fill_units(database(&opened)?, plan)?
    };
    let mut summary = FillSummaryView { filled: 0, failed: 0, stopped: false };

    for (index, unit) in units.iter().enumerate() {
        if session.stop_requested() {
            summary.stopped = true;
            break;
        }

        let (label, outcome) = fill_step(session, task, plan, *unit).await?;
        let (filled, notes) = match outcome {
            Outcome::Filled(notes) => (true, notes),
            Outcome::Failed(reason) => (false, vec![reason]),
        };

        if filled {
            summary.filled += 1;
        } else {
            summary.failed += 1;
        }

        on_step
            .send(FillStepView { number: index + 1, total: units.len(), label, filled, notes })
            .map_err(|error| ViewError::Invalid(format!("the page does not listen: {error}")))?;
    }

    Ok(summary)
}

/// Fills every parameter but the dependencies of every work unit of the
/// plan `plan`, over what was there.
///
/// Called by: `PlanFill.svelte`, "Fill parameters".
/// Calls: [`fill`].
#[tauri::command]
pub async fn plan_fill_parameters(
    session: State<'_, Session>,
    plan: i64,
    on_step: Channel<FillStepView>,
) -> Result<FillSummaryView, ViewError> {
    fill(&session, Task::Parameters, plan, &on_step).await
}

/// Fills the dependencies of every work unit of the plan `plan`, over what
/// was there; a dependency that closes a cycle is written and marked for
/// the planner.
///
/// Called by: `PlanFill.svelte`, "Fill dependencies".
/// Calls: [`fill`].
#[tauri::command]
pub async fn plan_fill_dependencies(
    session: State<'_, Session>,
    plan: i64,
    on_step: Channel<FillStepView>,
) -> Result<FillSummaryView, ViewError> {
    fill(&session, Task::Dependencies, plan, &on_step).await
}

/// What a filling of the plan `plan` would ask, counted now: the number of
/// questions, the agent and its integration, for the confirmation of the
/// planner.
///
/// Called by: `PlanFill.svelte`, before a filling starts.
/// Calls: `fill_preview`.
#[tauri::command]
pub fn plan_fill_preview(session: State<'_, Session>, plan: i64) -> Result<FillPreviewView, ViewError> {
    let opened = session.lock()?;
    let preview = fill_preview(database(&opened)?, plan)?;

    Ok(FillPreviewView { questions: preview.questions, agent: preview.agent, integration: preview.integration })
}

/// Asks the filling that runs to stop after its present work unit.
///
/// Called by: `PlanFill.svelte`, "Stop".
/// Calls: `Session::request_stop`.
#[tauri::command]
pub fn plan_fill_stop(session: State<'_, Session>) {
    session.request_stop();
}
