//! The data of the screen of an evaluation, as the page gets it: what the
//! package says of itself, the product, the tasks with their state, and the
//! agent with knowledge of the evaluation with those it may choose (step 57.4).

use serde::Serialize;

use crate::app::error::AppError;
use crate::db::evaluation as db_evaluation;
use crate::db::knowledge as db_knowledge;
use crate::db::user::database::UserDb;
use crate::model::evaluation::Evaluation;

/// An agent with knowledge that may be chosen.
#[derive(Debug, Serialize)]
pub struct AgentOptionView {
    pub id: i64,
    pub name: String,
}

/// A task of the evaluation.
#[derive(Debug, Serialize)]
pub struct LabTaskView {
    pub uuid: String,
    pub label: String,
    pub title: String,
    /// `"todo"`, `"working"` or `"done"`.
    pub state: &'static str,
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
    /// The agent of the evaluation; `None` when it has none.
    pub agent: Option<i64>,
    pub agents: Vec<AgentOptionView>,
}

/// The agents with knowledge of the evaluator, to be chosen.
///
/// Called by: [`screen`], `lab_task/read.rs` (the agent of a task).
/// Calls: `db_knowledge::read::agents`.
pub fn agents(db: &UserDb) -> Result<Vec<AgentOptionView>, AppError> {
    let summaries = db_knowledge::read::agents(db)?;
    let mut options = Vec::with_capacity(summaries.len());

    for summary in summaries {
        options.push(AgentOptionView { id: summary.id, name: summary.name });
    }

    Ok(options)
}

/// The screen of `evaluation`: its tasks, its agent and the agents to choose.
///
/// Called by: `lab_tasks_get`, `lab_tasks_state`, `lab_tasks_agent` (`evaluation_lab.rs`).
/// Calls: `db_evaluation::read::tasks`, `db_evaluation::read::agent`, [`agents`].
pub fn screen(db: &UserDb, evaluation: Evaluation) -> Result<EvaluationView, AppError> {
    let tasks = db_evaluation::read::tasks(db, &evaluation.uuid)?;
    let agent = db_evaluation::read::agent(db, &evaluation.uuid)?;
    let mut views = Vec::with_capacity(tasks.len());

    for task in tasks {
        views.push(LabTaskView { uuid: task.uuid, label: task.label, title: task.title, state: task.state.name() });
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
        agent,
        agents: agents(db)?,
    })
}
