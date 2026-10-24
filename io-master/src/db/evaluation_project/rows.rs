//! The rows of the master projects and the types of
//! `model/evaluation_project.rs` they become.

use crate::db::error::{Corrupt, DbError};
use crate::model::evaluation_project::{EvaluationTask, ProjectFields, ProjectSummary};
use crate::model::task_scheme::{Block, BlockFields, BlockKind, Connection, OutputCondition, OutputFormat};

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

/// A value of a column of `task_blocks` the program never writes.
fn corrupt(column: &'static str, value: String) -> DbError {
    DbError::Corrupt(Corrupt { table: "task_blocks", column, value })
}

/// A row of `task_blocks`.
///
/// Called by: `read::blocks`, `read::block`.
pub(super) fn block(row: &rusqlite::Row<'_>) -> Result<Block, DbError> {
    let kind: String = row.get("kind")?;
    let format: String = row.get("format")?;
    let condition: String = row.get("condition")?;

    let Some(kind) = BlockKind::from_name(&kind) else {
        return Err(corrupt("kind", kind));
    };
    let Some(format) = OutputFormat::from_name(&format) else {
        return Err(corrupt("format", format));
    };
    let Some(condition) = OutputCondition::from_name(&condition) else {
        return Err(corrupt("condition", condition));
    };

    Ok(Block {
        uuid: row.get("uuid")?,
        kind,
        x: row.get("x")?,
        y: row.get("y")?,
        fields: BlockFields {
            name: row.get("name")?,
            evidence: row.get("evidence")?,
            instructions: row.get("instructions")?,
            target: row.get("target")?,
            format,
            condition,
        },
    })
}

/// A row of `task_connections`.
///
/// Called by: `read::connections`.
pub(super) fn connection(row: &rusqlite::Row<'_>) -> Result<Connection, DbError> {
    Ok(Connection { from: row.get("from_block")?, to: row.get("to_block")? })
}
