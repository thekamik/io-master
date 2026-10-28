//! The data of the list of master projects, as the page gets it.

use serde::Serialize;

use crate::model_old::evaluation_project::ProjectSummary;

/// A master project, as a row of the list.
#[derive(Debug, Serialize)]
pub struct ProjectSummaryView {
    pub uuid: String,
    pub title: String,
    pub catalogs: usize,
    pub tasks: usize,
}

impl ProjectSummaryView {
    pub fn new(summary: ProjectSummary) -> ProjectSummaryView {
        ProjectSummaryView {
            uuid: summary.uuid,
            title: summary.title,
            catalogs: summary.catalogs,
            tasks: summary.tasks,
        }
    }
}

/// The views of `summaries`, in their order.
///
/// Called by: `project_list_get` (`evaluation_plan.rs`).
pub fn all(summaries: Vec<ProjectSummary>) -> Vec<ProjectSummaryView> {
    let mut views = Vec::with_capacity(summaries.len());

    for summary in summaries {
        views.push(ProjectSummaryView::new(summary));
    }

    views
}
