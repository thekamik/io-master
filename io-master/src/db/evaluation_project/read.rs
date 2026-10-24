//! Reading master projects and their tasks.

use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{
    EvaluationProject, EvaluationTask, MAX_CATALOGS, MAX_PROJECTS, MAX_TASKS, ProjectSummary,
};

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
    use crate::test_support::standards::database;

    #[test]
    fn the_list_counts_the_catalogs_and_the_tasks() {
        let test = database();
        let fields = ProjectFields { title: "CC".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c3".to_owned(), "c2".to_owned()]).unwrap();
        let added = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c3".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &added).unwrap();

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
