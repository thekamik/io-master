//! The data of the tasks the AI proposes for a master project (step 51.4 of
//! docs/history.md), as the page gets it: every proposal with its requirement
//! read from its catalog, and the reason of the AI.

use serde::Serialize;

use crate::app::error::AppError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{EvaluationTask, TaskProposal};

use super::super::task_list::view as task_view;

/// A requirement proposed as a task.
#[derive(Debug, Serialize)]
pub struct TaskProposalView {
    pub catalog_uuid: String,
    pub catalog_title: String,
    pub control_id: String,
    /// The identifier the standard writes, or the id.
    pub label: String,
    pub title: String,
    pub reason: String,
}

/// The views of `proposals`, their requirements read as the list of tasks
/// reads them.
///
/// Called by: the commands of the proposals (`evaluation_plan.rs`).
/// Calls: `task_view::all`.
pub fn all(db: &UserDb, proposals: Vec<TaskProposal>) -> Result<Vec<TaskProposalView>, AppError> {
    let mut requirements = Vec::with_capacity(proposals.len());

    // A proposal is no task yet: it has no UUID, which the list of tasks
    // only passes through.
    for proposal in &proposals {
        requirements.push(EvaluationTask {
            uuid: String::new(),
            catalog_uuid: proposal.catalog_uuid.clone(),
            control_id: proposal.control_id.clone(),
        });
    }

    let read = task_view::all(db, requirements)?;

    ensure!(read.len() == proposals.len(), AppError::Failed("a proposal was lost".to_owned()));

    let mut views = Vec::with_capacity(read.len());

    for (requirement, proposal) in read.into_iter().zip(proposals) {
        views.push(TaskProposalView {
            catalog_uuid: requirement.catalog_uuid,
            catalog_title: requirement.catalog_title,
            control_id: requirement.control_id,
            label: requirement.label,
            title: requirement.title,
            reason: proposal.reason,
        });
    }

    Ok(views)
}
