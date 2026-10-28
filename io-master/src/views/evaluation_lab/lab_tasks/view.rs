//! The data of the screen of an evaluation, as the page gets it: what the
//! package says of itself, the product, the tasks with their state and how
//! far each is (its verdict, its accepted fragments, its outputs ready, step
//! 59), the progress of the whole evaluation, and the agent with knowledge
//! of the evaluation with those it may choose (step 57.4).

use serde::Serialize;

use crate::app::error::AppError;
use crate::components::knowledge_agent::view::{AgentOptionView, agents};
use crate::db_old::evaluation as db_evaluation;
use crate::db_old::knowledge as db_knowledge;
use crate::db_old::user::database::UserDb;
use crate::model_old::evaluation::{Evaluation, TaskProgress, TaskState, Verdict};

/// A task of the evaluation, with how far it is.
#[derive(Debug, Serialize)]
pub struct LabTaskView {
    pub uuid: String,
    pub label: String,
    pub title: String,
    /// `"todo"`, `"working"` or `"done"`.
    pub state: &'static str,
    /// `"met"`, `"not_met"`, `"inconclusive"`, or none yet.
    pub verdict: Option<&'static str>,
    pub fragments: usize,
    /// The outputs its verdict makes due, and those of them whose text is accepted.
    pub outputs_due: usize,
    pub outputs_ready: usize,
}

/// The progress of the whole evaluation: the tasks by verdict, the tasks
/// done, the outputs due and ready for the report.
#[derive(Debug, Default, Serialize)]
pub struct ProgressView {
    pub tasks: usize,
    pub done: usize,
    pub met: usize,
    pub not_met: usize,
    pub inconclusive: usize,
    pub no_verdict: usize,
    pub outputs_due: usize,
    pub outputs_ready: usize,
}

/// An evaluation with its tasks.
#[derive(Debug, Serialize)]
pub struct EvaluationView {
    pub uuid: String,
    pub project_title: String,
    pub scope: String,
    pub package_name: String,
    pub evaluator: String,
    pub note: String,
    pub product: String,
    pub product_version: String,
    pub tasks: Vec<LabTaskView>,
    pub progress: ProgressView,
    /// The agent of the evaluation; `None` when it has none.
    pub agent: Option<i64>,
    pub agents: Vec<AgentOptionView>,
}

/// The screen of `evaluation`: its tasks with how far each is, its
/// progress, its agent and the agents to choose.
///
/// Called by: `lab_tasks_get`, `lab_tasks_state`, `lab_tasks_agent` (`evaluation_lab.rs`).
/// Calls: `db_evaluation::read::tasks`, `read::progress`, `read::agent`, [`counted`],
/// `agents` (`components/knowledge_agent/view.rs`).
pub fn screen(db: &UserDb, evaluation: Evaluation) -> Result<EvaluationView, AppError> {
    let tasks = db_evaluation::read::tasks(db, &evaluation.uuid)?;
    let progress = db_evaluation::read::progress(db, &evaluation.uuid)?;
    // An agent deleted since it was chosen is shown as none (it gives no passages either).
    let agent = match db_evaluation::read::agent(db, &evaluation.uuid)? {
        Some(id) if db_knowledge::read::agent(db, id)?.is_some() => Some(id),
        _ => None,
    };
    let mut summary = ProgressView::default();
    let mut views = Vec::with_capacity(tasks.len());

    for task in tasks {
        let done = task.state == TaskState::Done;
        let mut verdict = None;
        let mut view = LabTaskView {
            uuid: task.uuid,
            label: task.label,
            title: task.title,
            state: task.state.name(),
            verdict: None,
            fragments: 0,
            outputs_due: 0,
            outputs_ready: 0,
        };

        for found in &progress {
            if found.task == view.uuid {
                counted(&mut view, found);
                verdict = found.verdict;
            }
        }

        add(&mut summary, &view, verdict, done);
        views.push(view);
    }

    Ok(EvaluationView {
        uuid: evaluation.uuid,
        project_title: evaluation.project_title,
        scope: evaluation.scope,
        package_name: evaluation.package_name,
        evaluator: evaluation.evaluator,
        note: evaluation.note,
        product: evaluation.fields.product,
        product_version: evaluation.fields.product_version,
        tasks: views,
        progress: summary,
        agent,
        agents: agents(db)?,
    })
}

/// Writes into `view` how far its task is, from `progress`.
///
/// Called by: [`screen`].
/// Calls: `TaskProgress::outputs_due_and_ready`.
fn counted(view: &mut LabTaskView, progress: &TaskProgress) {
    let (due, ready) = progress.outputs_due_and_ready();

    view.verdict = match progress.verdict {
        Some(verdict) => Some(verdict.name()),
        None => None,
    };
    view.fragments = progress.accepted_fragments;
    view.outputs_due = due;
    view.outputs_ready = ready;
}

/// Adds the task `view`, of `verdict`, done or not, to the progress `summary`.
///
/// Called by: [`screen`].
fn add(summary: &mut ProgressView, view: &LabTaskView, verdict: Option<Verdict>, done: bool) {
    summary.tasks += 1;
    summary.outputs_due += view.outputs_due;
    summary.outputs_ready += view.outputs_ready;

    if done {
        summary.done += 1;
    }

    match verdict {
        Some(Verdict::Met) => summary.met += 1,
        Some(Verdict::NotMet) => summary.not_met += 1,
        Some(Verdict::Inconclusive) => summary.inconclusive += 1,
        None => summary.no_verdict += 1,
    }
}
