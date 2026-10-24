//! The data of the related elements of a task (step 51.4 of docs/history.md),
//! as the page gets it: every element with its requirement read from its
//! catalog, the reason of the AI, and whether it is only proposed.

use serde::Serialize;

use crate::app::error::AppError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{EvaluationTask, TaskLink};

use super::super::task_list::view as task_view;

/// A related element.
#[derive(Debug, Serialize)]
pub struct TaskLinkView {
    pub catalog_uuid: String,
    pub catalog_title: String,
    pub control_id: String,
    /// The identifier the standard writes, or the id.
    pub label: String,
    pub title: String,
    pub reason: String,
    /// Whether it is only proposed by the AI (drawn dashed).
    pub proposed: bool,
}

/// The views of `links`, their elements read as the list of tasks reads
/// requirements.
///
/// Called by: the commands of the related elements (`evaluation_plan.rs`).
/// Calls: `task_view::all`.
pub fn all(db: &UserDb, links: Vec<TaskLink>) -> Result<Vec<TaskLinkView>, AppError> {
    let mut elements = Vec::with_capacity(links.len());

    // An element is no task: it has no UUID, which the list of tasks only
    // passes through.
    for link in &links {
        elements.push(EvaluationTask {
            uuid: String::new(),
            catalog_uuid: link.catalog_uuid.clone(),
            control_id: link.control_id.clone(),
        });
    }

    let read = task_view::all(db, elements)?;

    ensure!(read.len() == links.len(), AppError::Failed("a related element was lost".to_owned()));

    let mut views = Vec::with_capacity(read.len());

    for (element, link) in read.into_iter().zip(links) {
        views.push(TaskLinkView {
            catalog_uuid: element.catalog_uuid,
            catalog_title: element.catalog_title,
            control_id: element.control_id,
            label: element.label,
            title: element.title,
            reason: link.reason,
            proposed: link.proposed,
        });
    }

    Ok(views)
}
