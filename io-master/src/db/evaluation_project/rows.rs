//! The rows of the master projects and the types of
//! `model/evaluation_project.rs` they become.

use crate::db::error::DbError;
use crate::model::evaluation_project::{EvaluationTask, ProjectFields, ProjectSummary};

/// A row of `evaluation_projects` with its counts (`read::projects`).
///
/// Called by: `read::projects`.
pub(super) fn summary(row: &rusqlite::Row<'_>) -> Result<ProjectSummary, DbError> {
    let catalogs: i64 = row.get("catalogs")?;
    let tasks: i64 = row.get("tasks")?;

    Ok(ProjectSummary {
        uuid: row.get("uuid")?,
        title: row.get("title")?,
        catalogs: usize::try_from(catalogs).map_err(|error| DbError::Inconsistent(error.to_string()))?,
        tasks: usize::try_from(tasks).map_err(|error| DbError::Inconsistent(error.to_string()))?,
    })
}

/// The fields of a row of `evaluation_projects`.
///
/// Called by: `read::project`.
pub(super) fn fields(row: &rusqlite::Row<'_>) -> Result<ProjectFields, DbError> {
    Ok(ProjectFields { title: row.get("title")?, description: row.get("description")? })
}

/// A row of `evaluation_tasks`.
///
/// Called by: `read::tasks`, `read::task`.
pub(super) fn task(row: &rusqlite::Row<'_>) -> Result<EvaluationTask, DbError> {
    Ok(EvaluationTask {
        uuid: row.get("uuid")?,
        catalog_uuid: row.get("catalog_uuid")?,
        control_id: row.get("control_id")?,
    })
}
