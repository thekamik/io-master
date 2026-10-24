//! Deleting master projects and their tasks.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes project `uuid` with its catalogs and tasks, in one transaction;
/// gives whether it existed.
///
/// Called by: `project_list_delete` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn project(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM evaluation_tasks WHERE project_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM project_catalogs WHERE project_uuid = ?1", [uuid])?;
    let deleted = transaction.execute("DELETE FROM evaluation_projects WHERE uuid = ?1", [uuid])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes task `uuid`; gives whether it existed.
///
/// Called by: `task_list_remove` (`evaluation_plan.rs`).
pub fn task(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM evaluation_tasks WHERE uuid = ?1", [uuid])? > 0)
}

#[cfg(test)]
mod tests {
    //! Tests of deleting projects and tasks.

    use super::*;

    use crate::db::evaluation_project::{create, read};
    use crate::model::evaluation_project::{EvaluationTask, ProjectFields};
    use crate::test_support::standards::database;

    #[test]
    fn a_project_goes_with_its_tasks_and_a_task_alone() {
        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned()]).unwrap();
        for uuid in ["t1", "t2"] {
            let task =
                EvaluationTask { uuid: uuid.to_owned(), catalog_uuid: "c".to_owned(), control_id: uuid.to_owned() };
            create::task(&test.db, "p", &task).unwrap();
        }

        assert!(task(&test.db, "t1").unwrap());
        assert_eq!(read::tasks(&test.db, "p").unwrap().len(), 1);

        assert!(project(&test.db, "p").unwrap());
        assert!(!project(&test.db, "p").unwrap());
        assert!(read::tasks(&test.db, "p").unwrap().is_empty());
        assert!(!read::catalog_in_use(&test.db, "c").unwrap());
    }
}
