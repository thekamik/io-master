//! Changing master projects: their fields and catalogs, the order of their
//! tasks.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{MAX_CATALOGS, ProjectFields};
use crate::model::time::now;

/// Replaces the fields and the catalogs of project `uuid`, in one
/// transaction.
///
/// Called by: `project_form_save` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn project(db: &UserDb, uuid: &str, fields: &ProjectFields, catalogs: &[String]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let changed = transaction.execute(
        "UPDATE evaluation_projects SET title = :title, description = :description, modified = :modified
         WHERE uuid = :uuid",
        named_params! { ":uuid": uuid, ":title": fields.title, ":description": fields.description, ":modified": now() },
    )?;

    if changed != 1 {
        return Err(DbError::NoRow { table: "evaluation_projects", id: 0 });
    }

    transaction.execute("DELETE FROM project_catalogs WHERE project_uuid = ?1", [uuid])?;

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

/// Adds catalog `catalog` at the end of the catalogs of project `project`,
/// when it does not use it already; refused when it uses `MAX_CATALOGS`.
///
/// Called by: `own_requirement_add` (`evaluation_plan.rs`): the catalog of the
/// laboratory joins the project with its first requirement there.
pub fn add_catalog(db: &UserDb, project: &str, catalog: &str) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let (count, there, last): (i64, bool, Option<i64>) = transaction.query_row(
        "SELECT COUNT(*), COALESCE(SUM(catalog_uuid = ?2), 0) > 0, MAX(position)
         FROM project_catalogs WHERE project_uuid = ?1",
        [project, catalog],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    if there {
        return Ok(());
    }

    let limit = i64::try_from(MAX_CATALOGS).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    if count >= limit {
        return Err(DbError::Inconsistent(format!("a project uses at most {MAX_CATALOGS} catalogs")));
    }

    transaction.execute(
        "INSERT INTO project_catalogs (project_uuid, catalog_uuid, position) VALUES (?1, ?2, ?3)",
        rusqlite::params![project, catalog, last.unwrap_or(-1) + 1],
    )?;
    transaction.commit()?;

    Ok(())
}

/// Moves task `uuid` one place up (towards the start) or down in its list,
/// by swapping it with its neighbour, in one transaction; gives whether it
/// moved (the first cannot go up, the last cannot go down).
///
/// Called by: `task_list_move` (`evaluation_plan.rs`).
pub fn move_task(db: &UserDb, uuid: &str, up: bool) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let (project, position): (String, i64) = transaction.query_row(
        "SELECT project_uuid, position FROM evaluation_tasks WHERE uuid = ?1",
        [uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let neighbour = if up {
        "SELECT uuid, position FROM evaluation_tasks WHERE project_uuid = ?1 AND position < ?2
         ORDER BY position DESC LIMIT 1"
    } else {
        "SELECT uuid, position FROM evaluation_tasks WHERE project_uuid = ?1 AND position > ?2
         ORDER BY position LIMIT 1"
    };

    let other: Option<(String, i64)> = transaction
        .query_row(neighbour, rusqlite::params![project, position], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;

    let Some((other, other_position)) = other else {
        return Ok(false);
    };

    transaction.execute(
        "UPDATE evaluation_tasks SET position = ?1 WHERE uuid = ?2",
        rusqlite::params![other_position, uuid],
    )?;
    transaction
        .execute("UPDATE evaluation_tasks SET position = ?1 WHERE uuid = ?2", rusqlite::params![position, other])?;
    transaction
        .execute("UPDATE evaluation_projects SET modified = ?1 WHERE uuid = ?2", rusqlite::params![now(), project])?;
    transaction.commit()?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    //! Tests of changing projects.

    use super::*;

    use crate::db::evaluation_project::{create, read};
    use crate::model::evaluation_project::EvaluationTask;
    use crate::test_support::standards::database;

    fn fields(title: &str) -> ProjectFields {
        ProjectFields { title: title.to_owned(), description: String::new() }
    }

    #[test]
    fn the_fields_and_the_catalogs_are_replaced() {
        let test = database();
        create::project(&test.db, "p", &fields("old"), &["a".to_owned()]).unwrap();

        project(&test.db, "p", &fields("new"), &["b".to_owned(), "a".to_owned()]).unwrap();

        let found = read::project(&test.db, "p").unwrap().unwrap();
        assert_eq!(
            (found.fields.title.as_str(), found.catalogs.clone()),
            ("new", vec!["b".to_owned(), "a".to_owned()])
        );
        assert!(project(&test.db, "none", &fields("x"), &[]).is_err());

        add_catalog(&test.db, "p", "lab").unwrap();
        add_catalog(&test.db, "p", "lab").unwrap();
        assert_eq!(read::project(&test.db, "p").unwrap().unwrap().catalogs, ["b", "a", "lab"]);
    }

    #[test]
    fn a_task_moves_by_one_place_and_not_past_the_ends() {
        let test = database();
        create::project(&test.db, "p", &fields("p"), &["c".to_owned()]).unwrap();
        for (uuid, control) in [("t1", "a"), ("t2", "b"), ("t3", "c")] {
            let task =
                EvaluationTask { uuid: uuid.to_owned(), catalog_uuid: "c".to_owned(), control_id: control.to_owned() };
            create::task(&test.db, "p", &task).unwrap();
        }

        assert!(move_task(&test.db, "t3", true).unwrap());
        assert!(!move_task(&test.db, "t1", true).unwrap());
        // After the move t2 is the last one.
        assert!(!move_task(&test.db, "t2", false).unwrap());

        let order: Vec<String> = read::tasks(&test.db, "p").unwrap().into_iter().map(|task| task.uuid).collect();
        assert_eq!(order, ["t1", "t3", "t2"]);
    }
}
