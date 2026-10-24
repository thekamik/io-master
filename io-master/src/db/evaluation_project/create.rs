//! Storing master projects and their tasks.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{EvaluationTask, MAX_LINKS, MAX_TASKS, ProjectFields, TaskLink, TaskProposal};
use crate::model::task_scheme::{Block, Correction, MAX_BLOCKS, MAX_OUTPUTS, NewScheme, StandardOutput};
use crate::model::time::now;
use crate::model::work_package::{MAX_PACKAGES, WorkPackage};

/// Stores a new project `uuid` with its fields and its catalogs, in one
/// transaction.
///
/// Called by: `new_project_create` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn project(db: &UserDb, uuid: &str, fields: &ProjectFields, catalogs: &[String]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO evaluation_projects (uuid, title, description, modified)
         VALUES (:uuid, :title, :description, :modified)",
        named_params! { ":uuid": uuid, ":title": fields.title, ":description": fields.description, ":modified": now() },
    )?;

    for (position, catalog) in catalogs.iter().enumerate() {
        let position = i64::try_from(position).map_err(|error| DbError::Inconsistent(error.to_string()))?;

        transaction.execute(
            "INSERT INTO project_catalogs (project_uuid, catalog_uuid, position) VALUES (:project, :catalog, :position)",
            named_params! { ":project": uuid, ":catalog": catalog, ":position": position },
        )?;
    }

    transaction.commit()?;

    Ok(())
}

/// Adds `task` at the end of the list of project `project` with its first
/// `scheme` (the block of its requirement and the outputs of the template of
/// the project, connected: `task_scheme::new_scheme`), in one transaction;
/// refused when the project holds [`MAX_TASKS`] already or the control is a
/// task of it already (the table keeps a control once). A proposal of the
/// AI for the same requirement is removed: it is decided.
///
/// Called by: `task_list_add`, `own_requirement_add`, `task_proposals_accept`
/// (`views/evaluation_plan/evaluation_plan.rs`).
pub fn task(db: &UserDb, project: &str, task: &EvaluationTask, scheme: &NewScheme) -> Result<(), DbError> {
    ensure!(
        scheme.blocks.len() <= MAX_BLOCKS,
        DbError::Inconsistent(format!("a task holds at most {MAX_BLOCKS} blocks"))
    );

    let transaction = db.connection().unchecked_transaction()?;

    let (count, last): (i64, Option<i64>) = transaction.query_row(
        "SELECT COUNT(*), MAX(position) FROM evaluation_tasks WHERE project_uuid = ?1",
        [project],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let limit = i64::try_from(MAX_TASKS).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    if count >= limit {
        return Err(DbError::Inconsistent(format!("a project holds at most {MAX_TASKS} tasks")));
    }

    transaction.execute(
        "INSERT INTO evaluation_tasks (uuid, project_uuid, catalog_uuid, control_id, position)
         VALUES (:uuid, :project, :catalog, :control, :position)",
        named_params! {
            ":uuid": task.uuid,
            ":project": project,
            ":catalog": task.catalog_uuid,
            ":control": task.control_id,
            ":position": last.unwrap_or(0) + 1,
        },
    )?;
    transaction.execute(
        "DELETE FROM task_proposals WHERE project_uuid = ?1 AND catalog_uuid = ?2 AND control_id = ?3",
        [project, &task.catalog_uuid, &task.control_id],
    )?;

    for block in &scheme.blocks {
        insert_block(&transaction, &task.uuid, block)?;
    }

    for connection in &scheme.connections {
        transaction.execute(
            "INSERT INTO task_connections (task_uuid, from_block, to_block) VALUES (?1, ?2, ?3)",
            [&task.uuid, &connection.from, &connection.to],
        )?;
    }

    transaction
        .execute("UPDATE evaluation_projects SET modified = ?1 WHERE uuid = ?2", rusqlite::params![now(), project])?;
    transaction.commit()?;

    Ok(())
}

/// Adds `block` (an input or an output, made by the planner or proposed by
/// the AI) to the scheme of task `task`; refused when the scheme holds
/// `MAX_BLOCKS` already.
///
/// Called by: `task_scheme_add_block` (`evaluation_plan.rs`),
/// `task_scheme/propose.rs`.
pub fn block(db: &UserDb, task: &str, block: &Block) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 =
        transaction.query_row("SELECT COUNT(*) FROM task_blocks WHERE task_uuid = ?1", [task], |row| row.get(0))?;
    let limit = i64::try_from(MAX_BLOCKS).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    if count >= limit {
        return Err(DbError::Inconsistent(format!("a task holds at most {MAX_BLOCKS} blocks")));
    }

    insert_block(&transaction, task, block)?;
    transaction.commit()?;

    Ok(())
}

/// Inserts `block` of task `task` in the transaction of the caller.
///
/// Called by: [`task`], [`block`].
fn insert_block(transaction: &rusqlite::Transaction<'_>, task: &str, block: &Block) -> Result<(), DbError> {
    transaction.execute(
        "INSERT INTO task_blocks (uuid, task_uuid, kind, x, y, name, evidence, instructions, target, format, condition,
                                  proposal)
         VALUES (:uuid, :task, :kind, :x, :y, :name, :evidence, :instructions, :target, :format, :condition,
                 :proposal)",
        named_params! {
            ":uuid": block.uuid,
            ":task": task,
            ":kind": block.kind.name(),
            ":x": block.x,
            ":y": block.y,
            ":name": block.fields.name,
            ":evidence": block.fields.evidence,
            ":instructions": block.fields.instructions,
            ":target": block.fields.target,
            ":format": block.fields.format.name(),
            ":condition": block.fields.condition.name(),
            ":proposal": block.proposal,
        },
    )?;

    Ok(())
}

/// Keeps `correction` of a block of task `task`, in place of one it may have
/// had.
///
/// Called by: `task_scheme/propose.rs`.
pub fn correction(db: &UserDb, task: &str, correction: &Correction) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT OR REPLACE INTO block_corrections (block_uuid, task_uuid, name, evidence, instructions, reason)
         VALUES (:block, :task, :name, :evidence, :instructions, :reason)",
        named_params! {
            ":block": correction.block,
            ":task": task,
            ":name": correction.name,
            ":evidence": correction.evidence,
            ":instructions": correction.instructions,
            ":reason": correction.reason,
        },
    )?;

    Ok(())
}

/// Adds `output` at the end of the template of the outputs of project
/// `project`; refused when the template holds [`MAX_OUTPUTS`] already (a
/// task could not take more).
///
/// Called by: `standard_outputs_add` (`evaluation_plan.rs`).
pub fn standard_output(db: &UserDb, project: &str, output: &StandardOutput) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let (count, last): (i64, Option<i64>) = transaction.query_row(
        "SELECT COUNT(*), MAX(position) FROM standard_outputs WHERE project_uuid = ?1",
        [project],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let limit = i64::try_from(MAX_OUTPUTS).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    if count >= limit {
        return Err(DbError::Inconsistent(format!("a project has at most {MAX_OUTPUTS} standard outputs")));
    }

    transaction.execute(
        "INSERT INTO standard_outputs (uuid, project_uuid, position, name, instructions, target, format, condition)
         VALUES (:uuid, :project, :position, :name, :instructions, :target, :format, :condition)",
        named_params! {
            ":uuid": output.uuid,
            ":project": project,
            ":position": last.unwrap_or(0) + 1,
            ":name": output.fields.name,
            ":instructions": output.fields.instructions,
            ":target": output.fields.target,
            ":format": output.fields.format.name(),
            ":condition": output.fields.condition.name(),
        },
    )?;
    transaction
        .execute("UPDATE evaluation_projects SET modified = ?1 WHERE uuid = ?2", rusqlite::params![now(), project])?;
    transaction.commit()?;

    Ok(())
}

/// Keeps `proposal` of the AI for project `project`, in place of an earlier
/// one for the same requirement.
///
/// Called by: `task_proposals/propose.rs`.
pub fn task_proposal(db: &UserDb, project: &str, proposal: &TaskProposal) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT OR REPLACE INTO task_proposals (project_uuid, catalog_uuid, control_id, reason)
         VALUES (:project, :catalog, :control, :reason)",
        named_params! {
            ":project": project,
            ":catalog": proposal.catalog_uuid,
            ":control": proposal.control_id,
            ":reason": proposal.reason,
        },
    )?;

    Ok(())
}

/// Adds `link` to the related elements of task `task`; an element related
/// already stays as it is. Refused when the task has [`MAX_LINKS`] already.
///
/// Called by: `task_links/propose.rs`.
pub fn task_link(db: &UserDb, task: &str, link: &TaskLink) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 =
        transaction.query_row("SELECT COUNT(*) FROM task_links WHERE task_uuid = ?1", [task], |row| row.get(0))?;
    let limit = i64::try_from(MAX_LINKS).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    if count >= limit {
        return Err(DbError::Inconsistent(format!("a task has at most {MAX_LINKS} related elements")));
    }

    transaction.execute(
        "INSERT OR IGNORE INTO task_links (task_uuid, catalog_uuid, control_id, reason, proposed)
         VALUES (:task, :catalog, :control, :reason, :proposed)",
        named_params! {
            ":task": task,
            ":catalog": link.catalog_uuid,
            ":control": link.control_id,
            ":reason": link.reason,
            ":proposed": link.proposed,
        },
    )?;
    transaction.commit()?;

    Ok(())
}

/// Stores `package` of project `project` with its tasks, in one
/// transaction; refused when the project has [`MAX_PACKAGES`] already or a
/// task is in another package (the table keeps a task once).
///
/// Called by: `work_packages_add` (`evaluation_plan.rs`).
pub fn work_package(db: &UserDb, project: &str, package: &WorkPackage) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let count: i64 =
        transaction
            .query_row("SELECT COUNT(*) FROM work_packages WHERE project_uuid = ?1", [project], |row| row.get(0))?;
    let limit = i64::try_from(MAX_PACKAGES).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    if count >= limit {
        return Err(DbError::Inconsistent(format!("a project has at most {MAX_PACKAGES} work packages")));
    }

    transaction.execute(
        "INSERT INTO work_packages (uuid, project_uuid, name, evaluator, note, modified)
         VALUES (:uuid, :project, :name, :evaluator, :note, :modified)",
        named_params! {
            ":uuid": package.uuid,
            ":project": project,
            ":name": package.fields.name,
            ":evaluator": package.fields.evaluator,
            ":note": package.fields.note,
            ":modified": now(),
        },
    )?;
    insert_package_tasks(&transaction, &package.uuid, &package.tasks)?;
    transaction.commit()?;

    Ok(())
}

/// Inserts `tasks` of package `package`, in their order, in the transaction
/// of the caller.
///
/// Called by: [`work_package`], `update::work_package`.
pub(super) fn insert_package_tasks(
    transaction: &rusqlite::Transaction<'_>,
    package: &str,
    tasks: &[String],
) -> Result<(), DbError> {
    for (position, task) in tasks.iter().enumerate() {
        let position = i64::try_from(position).map_err(|error| DbError::Inconsistent(error.to_string()))?;

        transaction.execute(
            "INSERT INTO package_tasks (task_uuid, package_uuid, position) VALUES (?1, ?2, ?3)",
            rusqlite::params![task, package, position],
        )?;
    }

    Ok(())
}

/// Connects block `from` to block `to` of task `task`; a connection made
/// twice is kept once.
///
/// Called by: `task_scheme_connect` (`evaluation_plan.rs`).
pub fn connection(db: &UserDb, task: &str, from: &str, to: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT OR IGNORE INTO task_connections (task_uuid, from_block, to_block) VALUES (?1, ?2, ?3)",
        [task, from, to],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of storing projects and tasks.

    use super::*;

    use crate::db::evaluation_project::read;
    use crate::model::task_scheme::{BlockFields, BlockKind, new_scheme};
    use crate::test_support::standards::database;

    fn fields() -> ProjectFields {
        ProjectFields { title: "CC:2022 R1".to_owned(), description: "All of Part 3".to_owned() }
    }

    fn a_task(uuid: &str, control: &str) -> EvaluationTask {
        EvaluationTask { uuid: uuid.to_owned(), catalog_uuid: "c3".to_owned(), control_id: control.to_owned() }
    }

    #[test]
    fn a_project_is_stored_with_its_catalogs_in_order() {
        let test = database();

        project(&test.db, "p", &fields(), &["c3".to_owned(), "c2".to_owned()]).unwrap();

        let found = read::project(&test.db, "p").unwrap().unwrap();
        assert_eq!(found.fields, fields());
        assert_eq!(found.catalogs, ["c3", "c2"]);
    }

    #[test]
    fn tasks_go_to_the_end_and_a_control_is_a_task_once() {
        let test = database();
        project(&test.db, "p", &fields(), &["c3".to_owned()]).unwrap();

        task(&test.db, "p", &a_task("t1", "adv_fsp.4"), &new_scheme("b1", &[]).unwrap()).unwrap();
        task(&test.db, "p", &a_task("t2", "ate_ind.2"), &new_scheme("b2", &[]).unwrap()).unwrap();

        let tasks = read::tasks(&test.db, "p").unwrap();
        assert_eq!((tasks[0].uuid.as_str(), tasks[1].uuid.as_str()), ("t1", "t2"));
        assert!(task(&test.db, "p", &a_task("t3", "adv_fsp.4"), &new_scheme("b3", &[]).unwrap()).is_err());
    }

    #[test]
    fn a_task_is_stored_with_the_outputs_of_its_first_scheme() {
        let test = database();
        project(&test.db, "p", &fields(), &["c3".to_owned()]).unwrap();
        let outputs = [("o1".to_owned(), BlockFields::empty()), ("o2".to_owned(), BlockFields::empty())];

        task(&test.db, "p", &a_task("t", "adv_fsp.4"), &new_scheme("r", &outputs).unwrap()).unwrap();

        let blocks = read::blocks(&test.db, "t").unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[2].kind, BlockKind::Output);
        assert_eq!(read::connections(&test.db, "t").unwrap().len(), 2);
    }

    #[test]
    fn the_standard_outputs_go_to_the_end_up_to_their_limit() {
        let test = database();
        project(&test.db, "p", &fields(), &["c3".to_owned()]).unwrap();

        for index in 0..MAX_OUTPUTS {
            let output = StandardOutput { uuid: format!("s{index}"), fields: BlockFields::empty() };
            standard_output(&test.db, "p", &output).unwrap();
        }

        let found = read::standard_outputs(&test.db, "p").unwrap();
        assert_eq!((found.len(), found[0].uuid.as_str()), (MAX_OUTPUTS, "s0"));
        let one_more = StandardOutput { uuid: "x".to_owned(), fields: BlockFields::empty() };
        assert!(standard_output(&test.db, "p", &one_more).is_err());
    }
}
