//! "Fill with AI", as the page calls it (user, 2026-10-29): the agent of
//! the planner fills every work unit of the plan, one after the other; two
//! commands, two tasks, each started by the planner on its own: the
//! parameters, then the dependencies (when the work units are described).
//! Each step is written into the progress of the session, which the page
//! asks for (`plan_fill_progress`), also when the panel is opened again; the
//! planner may stop the filling.
//!
//! ```text
//! PlanFill.svelte ──plan_fill_preview(plan)──► fill_preview: the questions counted now ──► FillPreviewView
//!                 ──plan_fill_parameters(plan)───┐
//!                 ──plan_fill_dependencies(plan)─┴► fill: fill_units, Session::begin_work, then
//!                     evaluation_plan_ai.rs: fill_plan (each work unit filled, its step
//!                     written into the progress)
//!                 ──plan_fill_progress(plan)──► progress_of ──► ProgressView
//!                 ──plan_fill_stop()──► the session asked to stop: the filling ends after its present step
//! ```
//!
//! Only one filling runs at a time (`Session::begin_work`). A work unit the
//! AI could not fill is told and passed over; a failure of the session or
//! of the database ends the filling.

use serde::Serialize;
use tauri::State;

use crate::objects::evaluation::evaluation_plan::evaluation_plan_ai::{fill_plan, fill_preview, fill_units};
use crate::objects::evaluation::work_unit::work_unit_ai::filling::FillTask;
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::panels::views::progress::{ProgressView, progress_of};

/// What a filling would ask, counted when the planner asks it.
#[derive(Serialize)]
pub struct FillPreviewView {
    questions: usize,
    agent: String,
    integration: String,
}

/// Fills every work unit of the plan `plan` in its order with the task
/// `task`, writing each step into the progress of the session. Refused
/// while another long work of the AI runs, or when the plan has no agent of
/// the planner.
///
/// Called by: [`plan_fill_parameters`], [`plan_fill_dependencies`].
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `fill_units`,
/// `fill_plan`; `Session::begin_work`, `Session::locks`.
async fn fill(session: &Session, task: FillTask, plan: i64) -> Result<(), ViewError> {
    let units = {
        let opened = session.lock()?;

        fill_units(database(opened.as_ref())?, plan)?
    };
    let Some(_work) = session.begin_work(Some(plan), units.len())? else {
        return Err(ViewError::Invalid("another work of the AI runs: wait for it or stop it".to_owned()));
    };
    let (database, progress) = session.locks();

    Ok(fill_plan(database, progress, task, plan, &units).await?)
}

/// Fills every parameter but the dependencies of every work unit of the
/// plan `plan`, over what was there.
///
/// Called by: `PlanFill.svelte`, "Fill parameters".
/// Calls: [`fill`].
#[tauri::command]
pub async fn plan_fill_parameters(session: State<'_, Session>, plan: i64) -> Result<(), ViewError> {
    fill(&session, FillTask::Parameters, plan).await
}

/// Fills the dependencies of every work unit of the plan `plan`, over what
/// was there; a dependency that closes a cycle is written and marked for
/// the planner.
///
/// Called by: `PlanFill.svelte`, "Fill dependencies".
/// Calls: [`fill`].
#[tauri::command]
pub async fn plan_fill_dependencies(session: State<'_, Session>, plan: i64) -> Result<(), ViewError> {
    fill(&session, FillTask::Dependencies, plan).await
}

/// What a filling of the plan `plan` would ask, counted now: the number of
/// questions, the agent and its integration, for the confirmation of the
/// planner.
///
/// Called by: `PlanFill.svelte`, before a filling starts.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `fill_preview`.
#[tauri::command]
pub fn plan_fill_preview(session: State<'_, Session>, plan: i64) -> Result<FillPreviewView, ViewError> {
    let opened = session.lock()?;
    let preview = fill_preview(database(opened.as_ref())?, plan)?;

    Ok(FillPreviewView { questions: preview.questions, agent: preview.agent, integration: preview.integration })
}

/// The progress of the last filling of the plan `plan`: while it runs, and
/// its summary once it ended; `None` when there was none since the start
/// of the program.
///
/// Called by: `PlanFill.svelte`, while a filling runs and when it is shown.
/// Calls: `progress.rs`: `progress_of`.
#[tauri::command]
pub fn plan_fill_progress(session: State<'_, Session>, plan: i64) -> Result<Option<ProgressView>, ViewError> {
    progress_of(&session, plan)
}

/// Asks the filling that runs to stop after its present work unit.
///
/// Called by: `PlanFill.svelte`, "Stop".
/// Calls: `Session::request_stop`.
#[tauri::command]
pub fn plan_fill_stop(session: State<'_, Session>) -> Result<(), ViewError> {
    Ok(session.request_stop()?)
}
