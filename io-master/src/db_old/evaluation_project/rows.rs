//! The rows of the master projects and the types of
//! `model/evaluation_project.rs` they become.

use crate::db_old::error::{Corrupt, DbError};
use crate::model_old::evaluation_project::{EvaluationTask, ProjectFields, ProjectSummary, TaskLink, TaskProposal};
use crate::model_old::task_scheme::{
    Block, BlockFields, BlockKind, Connection, Correction, OutputCondition, OutputFormat, StandardOutput,
};
use crate::model_old::work_package::PackageFields;

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

/// A row of `task_proposals`.
///
/// Called by: `read::task_proposals`.
pub(super) fn task_proposal(row: &rusqlite::Row<'_>) -> Result<TaskProposal, DbError> {
    Ok(TaskProposal {
        catalog_uuid: row.get("catalog_uuid")?,
        control_id: row.get("control_id")?,
        reason: row.get("reason")?,
    })
}

/// A row of `task_links`.
///
/// Called by: `read::task_links`.
pub(super) fn task_link(row: &rusqlite::Row<'_>) -> Result<TaskLink, DbError> {
    Ok(TaskLink {
        catalog_uuid: row.get("catalog_uuid")?,
        control_id: row.get("control_id")?,
        reason: row.get("reason")?,
        proposed: row.get("proposed")?,
    })
}

/// The fields of a row of `work_packages`.
///
/// Called by: `read::work_packages`, `read::work_package`.
pub(super) fn package_fields(row: &rusqlite::Row<'_>) -> Result<PackageFields, DbError> {
    Ok(PackageFields { name: row.get("name")?, evaluator: row.get("evaluator")?, note: row.get("note")? })
}

/// A value of a column of `table` the program never writes.
fn corrupt(table: &'static str, column: &'static str, value: String) -> DbError {
    DbError::Corrupt(Corrupt { table, column, value })
}

/// The format and the condition of an output in a row of `table`.
///
/// Called by: [`block`], [`standard_output`].
fn output_rules(row: &rusqlite::Row<'_>, table: &'static str) -> Result<(OutputFormat, OutputCondition), DbError> {
    let format: String = row.get("format")?;
    let condition: String = row.get("condition")?;

    let Some(format) = OutputFormat::from_name(&format) else {
        return Err(corrupt(table, "format", format));
    };
    let Some(condition) = OutputCondition::from_name(&condition) else {
        return Err(corrupt(table, "condition", condition));
    };

    Ok((format, condition))
}

/// A row of `task_blocks`.
///
/// Called by: `read::blocks`, `read::block`.
pub(super) fn block(row: &rusqlite::Row<'_>) -> Result<Block, DbError> {
    let kind: String = row.get("kind")?;

    let Some(kind) = BlockKind::from_name(&kind) else {
        return Err(corrupt("task_blocks", "kind", kind));
    };

    let (format, condition) = output_rules(row, "task_blocks")?;

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
        proposal: row.get("proposal")?,
    })
}

/// A row of `block_corrections`.
///
/// Called by: `read::corrections`, `read::correction`.
pub(super) fn correction(row: &rusqlite::Row<'_>) -> Result<Correction, DbError> {
    Ok(Correction {
        block: row.get("block_uuid")?,
        name: row.get("name")?,
        evidence: row.get("evidence")?,
        instructions: row.get("instructions")?,
        reason: row.get("reason")?,
    })
}

/// A row of `standard_outputs`.
///
/// Called by: `read::standard_outputs`, `read::standard_output`.
pub(super) fn standard_output(row: &rusqlite::Row<'_>) -> Result<StandardOutput, DbError> {
    let (format, condition) = output_rules(row, "standard_outputs")?;

    Ok(StandardOutput {
        uuid: row.get("uuid")?,
        fields: BlockFields {
            name: row.get("name")?,
            evidence: String::new(),
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
