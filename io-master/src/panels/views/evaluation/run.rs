//! The assessment of an evaluation by the AI, as the page calls it (user,
//! 2026-10-30): every work unit in the order of the work, one after the
//! other, without the evaluator. Each step is written into the progress of
//! the session, which the page asks for (`evaluation_assess_progress`), also
//! when the panel is opened again; the evaluator may stop the assessment.
//!
//! ```text
//! EvaluationRun.svelte ──evaluation_assess_preview(id, again)─► preview ──► AssessPreviewView
//!                      ──evaluation_assess(id, again)─► units_ready (refused while an input is missing),
//!                          Session::begin_work, then objects/evaluation/evaluation_plan/evaluation_plan_ai.rs:
//!                          assess_evaluation (each work unit assessed, its step written into the progress)
//!                      ──evaluation_assess_progress(id)────► progress_of ──► ProgressView
//!                      ──evaluation_assess_stop()──────────► the answer awaited ends, not taken; no further work unit
//! ```
//!
//! Only one long work of the AI runs at a time (`Session::begin_work`); a
//! work unit the AI could not assess is told and passed over; a failure of
//! the session or of the database ends the assessment.

use serde::Serialize;
use tauri::State;

use crate::objects::evaluation::evaluation_plan::evaluation_plan_ai::{
    Preview, assess_evaluation, preview, units_ready,
};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::panels::views::progress::{ProgressView, progress_of};

/// A name and how many questions or work units it counts.
#[derive(Serialize)]
pub struct CountView {
    name: String,
    count: usize,
}

/// What an assessment would ask, counted when the evaluator asks it.
#[derive(Serialize)]
pub struct AssessPreviewView {
    questions: usize,
    agents: Vec<CountView>,
    /// The documents and proofs needed and not given: the assessment does
    /// not start while there is one.
    missing: Vec<CountView>,
}

/// The counts of `counted` as the page shows them.
///
/// Input: `counted`, names with their counts.
/// Output: their views (`[CountView { name: "Evaluator", count: 2 }]`).
///
/// Called by: [`preview_view`].
fn count_views(counted: Vec<(String, usize)>) -> Vec<CountView> {
    let mut views = Vec::with_capacity(counted.len());

    for (name, count) in counted {
        views.push(CountView { name, count });
    }

    views
}

/// The preview `found` as the page shows it.
///
/// Input: `found`, the preview of an assessment.
/// Output: its view, the work units counted.
///
/// Called by: [`evaluation_assess_preview`].
/// Calls: [`count_views`].
fn preview_view(found: Preview) -> AssessPreviewView {
    AssessPreviewView {
        questions: found.units.len(),
        agents: count_views(found.agents),
        missing: count_views(found.missing),
    }
}

/// What the assessment of the evaluation `id` would ask, counted now; with
/// `again` the work units of the AI assessed already too.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an evaluation in progress; `again`, whether the work units with a verdict
/// are assessed again.
/// Output: the preview (`{"units": 12, "agents": [...], "missing": []}`);
/// `ViewError::NotLoggedIn` when nobody is logged in, the refusals of the
/// evaluation.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationAssessPreview`, from
/// `ui/src/views/evaluation/EvaluationUnits.svelte`, before an assessment starts.
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `preview`; [`preview_view`].
#[tauri::command]
pub fn evaluation_assess_preview(
    session: State<'_, Session>,
    id: i64,
    again: bool,
) -> Result<AssessPreviewView, ViewError> {
    let opened = session.lock()?;

    Ok(preview_view(preview(database(opened.as_ref())?, id, again)?))
}

/// Assesses every work unit to assess of the evaluation `id` in the order of
/// the work, writing each step into the progress of the session; with
/// `again` the work units of the AI assessed already too. Refused while
/// another work of the AI runs, or an input is missing.
///
/// Input: `session`, the session of the window, someone logged in; `id`, the id
/// of an evaluation in progress; `again`, as [`evaluation_assess_preview`].
/// Output: nothing once the assessment ended; `ViewError::NotLoggedIn` when
/// nobody is logged in, `ViewError::Invalid` while another work of the AI runs,
/// the refusals of the evaluation (an input missing), an error of the session
/// or the database.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationAssess`, from
/// `ui/src/views/evaluation/EvaluationRun.svelte`, "Start".
/// Calls: `panels/session.rs`: `Session::lock`; `panels/views/opened.rs`: `database`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `units_ready`, `assess_evaluation`;
/// `panels/session.rs`: `Session::begin_work`, `Session::locks`.
#[tauri::command]
pub async fn evaluation_assess(session: State<'_, Session>, id: i64, again: bool) -> Result<(), ViewError> {
    let units = {
        let opened = session.lock()?;

        units_ready(database(opened.as_ref())?, id, again)?
    };
    let Some(_work) = session.begin_work(Some(id), units.len())? else {
        return Err(ViewError::Invalid("another work of the AI runs: wait for it or stop it".to_owned()));
    };
    let (database, progress) = session.locks();

    Ok(assess_evaluation(database, progress, id, &units).await?)
}

/// The progress of the last assessment of the evaluation `id`: while it
/// runs, and its summary once it ended; `None` when there was none since
/// the start of the program.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationAssessProgress`, from
/// `ui/src/views/evaluation/EvaluationRun.svelte`, while an assessment runs and when it is shown.
/// Calls: `panels/views/progress.rs`: `progress_of`.
#[tauri::command]
pub fn evaluation_assess_progress(session: State<'_, Session>, id: i64) -> Result<Option<ProgressView>, ViewError> {
    progress_of(&session, id)
}

/// Asks the assessment that runs to stop: the answer it awaits ends at once and is
/// not taken, no further work unit is begun.
///
/// Called by: `ui/src/lib/api/evaluation.ts`: `evaluationAssessStop`, from
/// `ui/src/views/evaluation/EvaluationUnits.svelte`, "Stop".
/// Calls: `panels/session.rs`: `Session::request_stop`.
#[tauri::command]
pub fn evaluation_assess_stop(session: State<'_, Session>) -> Result<(), ViewError> {
    Ok(session.request_stop()?)
}
