//! The data of the screen of an evaluation, as the page gets it: what the
//! package says of itself, the product, and the tasks with their state.

use serde::Serialize;

use crate::model::evaluation::{Evaluation, LabTask};

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
}

/// The view of `evaluation` with `tasks`.
///
/// Called by: `lab_tasks_get`, `lab_tasks_state` (`evaluation_lab.rs`).
pub fn new(evaluation: Evaluation, tasks: Vec<LabTask>) -> EvaluationView {
    let mut views = Vec::with_capacity(tasks.len());

    for task in tasks {
        views.push(LabTaskView { uuid: task.uuid, label: task.label, title: task.title, state: task.state.name() });
    }

    EvaluationView {
        uuid: evaluation.uuid,
        project_title: evaluation.project_title,
        scope: evaluation.scope,
        package_name: evaluation.package_name,
        evaluator: evaluation.evaluator,
        note: evaluation.note,
        product: evaluation.fields.product,
        product_version: evaluation.fields.product_version,
        tasks: views,
    }
}
