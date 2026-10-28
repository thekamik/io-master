//! The data of the list of evaluations, as the page gets it.

use serde::Serialize;

use crate::model_old::evaluation::EvaluationSummary;

/// An evaluation as a row of the list, with its progress.
#[derive(Debug, Serialize)]
pub struct EvaluationSummaryView {
    pub uuid: String,
    pub project_title: String,
    pub package_name: String,
    pub product: String,
    pub tasks: usize,
    pub done: usize,
}

/// The evaluations as the page shows them.
///
/// Called by: `evaluation_list_get`, `evaluation_list_delete`, `package_import_import` (`evaluation_lab.rs`).
pub fn all(summaries: Vec<EvaluationSummary>) -> Vec<EvaluationSummaryView> {
    let mut views = Vec::with_capacity(summaries.len());

    for summary in summaries {
        views.push(EvaluationSummaryView {
            uuid: summary.uuid,
            project_title: summary.project_title,
            package_name: summary.package_name,
            product: summary.product,
            tasks: summary.tasks,
            done: summary.done,
        });
    }

    views
}
