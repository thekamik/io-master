//! The progress of the long work of the AI as the page shows it (user,
//! 2026-11-01): the page asks for it while the work runs, and again when a
//! panel is opened, so that the bar of a work that runs is shown again.
//!
//! ```text
//! EvaluationRun.svelte ──evaluation_assess_progress(id)─┐
//! PlanFill.svelte      ──plan_fill_progress(plan)───────┴► progress_of ──► Session::progress ──► ProgressView
//!                                                                          (None: no work on this plan)
//! ```

use serde::Serialize;

use crate::ai::models::evaluation::work_unit::assessment_question::verdict_name;
use crate::model::session::progress::Progress;
use crate::panels::views::error::ViewError;
use crate::session::state::Session;

/// A work unit not taken, or taken with notes, as the page shows it.
#[derive(Serialize)]
pub struct ProblemView {
    label: String,
    notes: Vec<String>,
}

/// The progress of the last long work on one plan or evaluation.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressView {
    /// It runs; otherwise it ended and the counts are its summary.
    running: bool,
    /// The user asked it to stop.
    stopping: bool,
    total: usize,
    assessed: usize,
    failed: usize,
    /// The last work unit gone through; empty before the first.
    label: String,
    /// "pass", "fail", "inconclusive"; `None` without a verdict.
    last_verdict: Option<&'static str>,
    problems: Vec<ProblemView>,
}

/// What the page shows of `progress`.
///
/// Called by: [`progress_of`].
fn progress_view(progress: Progress) -> ProgressView {
    let mut problems = Vec::with_capacity(progress.problems.len());

    for problem in progress.problems {
        problems.push(ProblemView { label: problem.label, notes: problem.notes });
    }

    ProgressView {
        running: progress.running,
        stopping: progress.stop,
        total: progress.total,
        assessed: progress.assessed,
        failed: progress.failed,
        label: progress.label,
        last_verdict: progress.last_verdict.map(verdict_name),
        problems,
    }
}

/// The progress of the last long work of the session, when it was on the
/// plan or evaluation `evaluation_plan`; `None` when it was on another, or
/// there was none since the start of the program.
///
/// Called by: `panels/views/evaluation/run.rs`: `evaluation_assess_progress`;
/// `panels/views/evaluation_plan/fill.rs`: `plan_fill_progress`.
/// Calls: `Session::progress`, [`progress_view`].
pub fn progress_of(session: &Session, evaluation_plan: i64) -> Result<Option<ProgressView>, ViewError> {
    ensure!(evaluation_plan > 0, ViewError::Invalid(format!("no plan {evaluation_plan}")));

    let progress = session.progress()?.clone();

    if progress.evaluation_plan != Some(evaluation_plan) {
        return Ok(None);
    }

    Ok(Some(progress_view(progress)))
}
