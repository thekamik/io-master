//! Reading master projects and their tasks.

use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{
    EvaluationProject, EvaluationTask, MAX_CATALOGS, MAX_LINKS, MAX_PROJECTS, MAX_TASKS, ProjectSummary, TaskLink,
    TaskProposal,
};
use crate::model::task_scheme::{Block, Connection, Correction, MAX_BLOCKS, MAX_OUTPUTS, StandardOutput};
use crate::model::work_package::{MAX_PACKAGES, WorkPackage};

use super::rows;

/// A limit as a parameter of SQL.
fn limit(max: usize) -> Result<i64, DbError> {
    i64::try_from(max).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// The projects of the evaluator with the numbers of their catalogs and
/// tasks, the newest change first; at most `MAX_PROJECTS`.
///
/// Called by: `project_list_get` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn projects(db: &UserDb) -> Result<Vec<ProjectSummary>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT p.uuid AS uuid, p.title AS title,
                (SELECT COUNT(*) FROM project_catalogs c WHERE c.project_uuid = p.uuid) AS catalogs,
                (SELECT COUNT(*) FROM evaluation_tasks t WHERE t.project_uuid = p.uuid) AS tasks
         FROM evaluation_projects p ORDER BY p.modified DESC, p.title LIMIT ?1",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map([limit(MAX_PROJECTS)?], |row| Ok(rows::summary(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// Project `uuid` with its catalogs; `None` when it does not exist.
///
/// Called by: the commands of a project (`evaluation_plan.rs`).
pub fn project(db: &UserDb, uuid: &str) -> Result<Option<EvaluationProject>, DbError> {
    let connection = db.connection();
    let fields = connection
        .query_row("SELECT title, description FROM evaluation_projects WHERE uuid = ?1", [uuid], |row| {
            Ok(rows::fields(row))
        })
        .optional()?
        .transpose()?;

    let Some(fields) = fields else {
        return Ok(None);
    };

    let mut statement = connection
        .prepare("SELECT catalog_uuid FROM project_catalogs WHERE project_uuid = ?1 ORDER BY position LIMIT ?2")?;
    let mut catalogs = Vec::new();

    for catalog in statement.query_map(rusqlite::params![uuid, limit(MAX_CATALOGS)?], |row| row.get(0))? {
        catalogs.push(catalog?);
    }

    Ok(Some(EvaluationProject { uuid: uuid.to_owned(), fields, catalogs }))
}

/// The tasks of project `project`, in their order; at most `MAX_TASKS`.
///
/// Called by: `task_list_get` (`evaluation_plan.rs`).
pub fn tasks(db: &UserDb, project: &str) -> Result<Vec<EvaluationTask>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM evaluation_tasks WHERE project_uuid = ?1 ORDER BY position LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![project, limit(MAX_TASKS)?], |row| Ok(rows::task(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The requirements the AI proposes as tasks of project `project`, by
/// catalog and in the order proposed; at most `MAX_TASKS`.
///
/// Called by: `task_proposals_get` and the commands of the proposals (`evaluation_plan.rs`).
pub fn task_proposals(db: &UserDb, project: &str) -> Result<Vec<TaskProposal>, DbError> {
    let connection = db.connection();
    let mut statement = connection
        .prepare("SELECT * FROM task_proposals WHERE project_uuid = ?1 ORDER BY catalog_uuid, rowid LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![project, limit(MAX_TASKS)?], |row| Ok(rows::task_proposal(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The related elements of task `task`, in the order added; at most
/// `MAX_LINKS`.
///
/// Called by: `task_links_get` and the commands of the related elements (`evaluation_plan.rs`),
/// `task_links/propose.rs`.
pub fn task_links(db: &UserDb, task: &str) -> Result<Vec<TaskLink>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT * FROM task_links WHERE task_uuid = ?1 ORDER BY rowid LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![task, limit(MAX_LINKS)?], |row| Ok(rows::task_link(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The work packages of project `project` with their tasks, by name; at
/// most `MAX_PACKAGES`.
///
/// Called by: `work_packages_get` and the commands of the packages (`evaluation_plan.rs`).
/// Calls: [`package_tasks`].
pub fn work_packages(db: &UserDb, project: &str) -> Result<Vec<WorkPackage>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM work_packages WHERE project_uuid = ?1 ORDER BY name, rowid LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![project, limit(MAX_PACKAGES)?], |row| {
        Ok((row.get::<_, String>("uuid"), rows::package_fields(row)))
    })? {
        let (uuid, fields) = row?;
        let uuid = uuid?;
        let tasks = package_tasks(db, &uuid)?;

        found.push(WorkPackage { uuid, fields: fields?, tasks });
    }

    Ok(found)
}

/// Work package `uuid` with its tasks, and its project; `None` when it does
/// not exist.
///
/// Called by: `work_packages_save`, `work_packages_delete`, `work_packages_export` (`evaluation_plan.rs`).
/// Calls: [`package_tasks`].
pub fn work_package(db: &UserDb, uuid: &str) -> Result<Option<(String, WorkPackage)>, DbError> {
    let found = db
        .connection()
        .query_row("SELECT * FROM work_packages WHERE uuid = ?1", [uuid], |row| {
            Ok((row.get::<_, String>("project_uuid"), rows::package_fields(row)))
        })
        .optional()?;

    let Some((project, fields)) = found else {
        return Ok(None);
    };

    let package = WorkPackage { uuid: uuid.to_owned(), fields: fields?, tasks: package_tasks(db, uuid)? };

    Ok(Some((project?, package)))
}

/// The tasks of package `package`, in their order; at most `MAX_TASKS`.
///
/// Called by: [`work_packages`], [`work_package`].
fn package_tasks(db: &UserDb, package: &str) -> Result<Vec<String>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT task_uuid FROM package_tasks WHERE package_uuid = ?1 ORDER BY position LIMIT ?2")?;
    let mut tasks = Vec::new();

    for task in statement.query_map(rusqlite::params![package, limit(MAX_TASKS)?], |row| row.get(0))? {
        tasks.push(task?);
    }

    Ok(tasks)
}

/// The name of the package task `task` is in; `None` when it is in none.
///
/// Called by: `task_list/view.rs`.
pub fn package_of_task(db: &UserDb, task: &str) -> Result<Option<String>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT w.name FROM package_tasks p JOIN work_packages w ON w.uuid = p.package_uuid WHERE p.task_uuid = ?1",
            [task],
            |row| row.get(0),
        )
        .optional()?)
}

/// Task `uuid` and the project it is in; `None` when it does not exist.
///
/// Called by: `task_list_remove`, `task_list_move` (`evaluation_plan.rs`).
pub fn task(db: &UserDb, uuid: &str) -> Result<Option<(String, EvaluationTask)>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM evaluation_tasks WHERE uuid = ?1")?;

    let found =
        statement.query_row([uuid], |row| Ok((row.get::<_, String>("project_uuid"), rows::task(row)))).optional()?;

    match found {
        Some((project, task)) => Ok(Some((project?, task?))),
        None => Ok(None),
    }
}

/// Whether a project uses catalog `catalog` (it cannot be deleted then).
///
/// Called by: `oscal_documents_delete` (`evaluation_plan.rs`).
pub fn catalog_in_use(db: &UserDb, catalog: &str) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT COUNT(*) > 0 FROM project_catalogs WHERE catalog_uuid = ?1",
        [catalog],
        |row| row.get(0),
    )?)
}

/// Whether a task of project `project` points to catalog `catalog` (it
/// cannot leave the project then).
///
/// Called by: `project_form_save` (`evaluation_plan.rs`).
pub fn catalog_has_tasks(db: &UserDb, project: &str, catalog: &str) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT COUNT(*) > 0 FROM evaluation_tasks WHERE project_uuid = ?1 AND catalog_uuid = ?2",
        [project, catalog],
        |row| row.get(0),
    )?)
}

/// The template of the outputs of project `project`, in its order; at most
/// `MAX_OUTPUTS`.
///
/// Called by: `standard_outputs_get` and the commands that change the
/// template (`evaluation_plan.rs`), `task_scheme/scheme.rs` (`first`).
pub fn standard_outputs(db: &UserDb, project: &str) -> Result<Vec<StandardOutput>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM standard_outputs WHERE project_uuid = ?1 ORDER BY position LIMIT ?2")?;
    let mut found = Vec::new();

    for row in
        statement.query_map(rusqlite::params![project, limit(MAX_OUTPUTS)?], |row| Ok(rows::standard_output(row)))?
    {
        found.push(row??);
    }

    Ok(found)
}

/// Standard output `uuid` and its project; `None` when it does not exist.
///
/// Called by: `standard_outputs_save`, `standard_outputs_delete` (`evaluation_plan.rs`).
pub fn standard_output(db: &UserDb, uuid: &str) -> Result<Option<(String, StandardOutput)>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM standard_outputs WHERE uuid = ?1")?;

    let found = statement
        .query_row([uuid], |row| Ok((row.get::<_, String>("project_uuid"), rows::standard_output(row))))
        .optional()?;

    match found {
        Some((project, output)) => Ok(Some((project?, output?))),
        None => Ok(None),
    }
}

/// The blocks of the scheme of task `task`: its requirement first, then the
/// inputs and the outputs in the order they were added; at most
/// `MAX_BLOCKS`.
///
/// Called by: `task_scheme_get` and the commands that change a scheme
/// (`evaluation_plan.rs`).
pub fn blocks(db: &UserDb, task: &str) -> Result<Vec<Block>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(
        "SELECT * FROM task_blocks WHERE task_uuid = ?1
         ORDER BY kind = 'requirement' DESC, rowid LIMIT ?2",
    )?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![task, limit(MAX_BLOCKS)?], |row| Ok(rows::block(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The connections of the blocks of task `task`.
///
/// Called by: `task_scheme_get` and the commands that change a scheme.
pub fn connections(db: &UserDb, task: &str) -> Result<Vec<Connection>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT * FROM task_connections WHERE task_uuid = ?1 LIMIT ?2")?;
    let max = MAX_BLOCKS * MAX_BLOCKS;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![task, limit(max)?], |row| Ok(rows::connection(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The corrections the AI proposed for the blocks of task `task`; at most
/// one per block, so at most `MAX_BLOCKS`.
///
/// Called by: `scheme::read`, `task_scheme_accept_all` (`evaluation_plan.rs`).
pub fn corrections(db: &UserDb, task: &str) -> Result<Vec<Correction>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM block_corrections WHERE task_uuid = ?1 ORDER BY rowid LIMIT ?2")?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![task, limit(MAX_BLOCKS)?], |row| Ok(rows::correction(row)))? {
        found.push(row??);
    }

    Ok(found)
}

/// The correction proposed for block `block`; `None` when there is none.
///
/// Called by: `task_scheme_accept`, `task_scheme_reject` (`evaluation_plan.rs`).
pub fn correction(db: &UserDb, block: &str) -> Result<Option<Correction>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM block_corrections WHERE block_uuid = ?1")?;

    statement.query_row([block], |row| Ok(rows::correction(row))).optional()?.transpose()
}

/// Block `uuid` and the task it belongs to; `None` when it does not exist.
///
/// Called by: the commands that change a block (`evaluation_plan.rs`).
pub fn block(db: &UserDb, uuid: &str) -> Result<Option<(String, Block)>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM task_blocks WHERE uuid = ?1")?;
    let found =
        statement.query_row([uuid], |row| Ok((row.get::<_, String>("task_uuid"), rows::block(row)))).optional()?;

    match found {
        Some((task, block)) => Ok(Some((task?, block?))),
        None => Ok(None),
    }
}

/// Whether a task of any project points to control `control` of catalog
/// `catalog` (a requirement of the laboratory is not deleted then).
///
/// Called by: `own_requirement_delete` (`evaluation_plan.rs`).
pub fn control_in_use(db: &UserDb, catalog: &str, control: &str) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT COUNT(*) > 0 FROM evaluation_tasks WHERE catalog_uuid = ?1 AND control_id = ?2",
        [catalog, control],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    //! Tests of reading projects and tasks.

    use super::*;

    use crate::db::evaluation_project::create;
    use crate::model::evaluation_project::ProjectFields;
    use crate::model::task_scheme::new_scheme;
    use crate::test_support::standards::database;

    #[test]
    fn the_list_counts_the_catalogs_and_the_tasks() {
        let test = database();
        let fields = ProjectFields { title: "CC".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c3".to_owned(), "c2".to_owned()]).unwrap();
        let added = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c3".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &added, &new_scheme(&format!("b-{}", added.uuid), &[]).unwrap()).unwrap();

        let list = projects(&test.db).unwrap();

        assert_eq!((list[0].catalogs, list[0].tasks), (2, 1));
        assert_eq!(task(&test.db, "t").unwrap().unwrap().0, "p");
        assert!(catalog_in_use(&test.db, "c2").unwrap());
        assert!(!catalog_in_use(&test.db, "other").unwrap());
        assert!(catalog_has_tasks(&test.db, "p", "c3").unwrap());
        assert!(!catalog_has_tasks(&test.db, "p", "c2").unwrap());
        assert_eq!(project(&test.db, "nothing").unwrap(), None);
        assert!(control_in_use(&test.db, "c3", "x").unwrap());
        assert!(!control_in_use(&test.db, "c2", "x").unwrap());
    }
}
