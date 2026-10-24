//! Storing master projects and their tasks.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{EvaluationTask, MAX_TASKS, ProjectFields};
use crate::model::time::now;

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

/// Adds `task` at the end of the list of project `project`, in one
/// transaction; refused when the project holds [`MAX_TASKS`] already or the
/// control is a task of it already (the table keeps a control once).
///
/// Called by: `task_list_add` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn task(db: &UserDb, project: &str, task: &EvaluationTask) -> Result<(), DbError> {
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
    transaction
        .execute("UPDATE evaluation_projects SET modified = ?1 WHERE uuid = ?2", rusqlite::params![now(), project])?;
    transaction.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of storing projects and tasks.

    use super::*;

    use crate::db::evaluation_project::read;
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

        task(&test.db, "p", &a_task("t1", "adv_fsp.4")).unwrap();
        task(&test.db, "p", &a_task("t2", "ate_ind.2")).unwrap();

        let tasks = read::tasks(&test.db, "p").unwrap();
        assert_eq!((tasks[0].uuid.as_str(), tasks[1].uuid.as_str()), ("t1", "t2"));
        assert!(task(&test.db, "p", &a_task("t3", "adv_fsp.4")).is_err());
    }
}
