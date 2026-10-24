//! Deleting master projects and their tasks.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes project `uuid` with its catalogs, its template of outputs, its
/// tasks and their schemes, in one transaction; gives whether it existed.
///
/// Called by: `project_list_delete` (`views/evaluation_plan/evaluation_plan.rs`).
pub fn project(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM task_connections WHERE task_uuid IN (SELECT uuid FROM evaluation_tasks WHERE project_uuid = ?1)",
        [uuid],
    )?;
    transaction.execute(
        "DELETE FROM task_blocks WHERE task_uuid IN (SELECT uuid FROM evaluation_tasks WHERE project_uuid = ?1)",
        [uuid],
    )?;
    transaction.execute("DELETE FROM evaluation_tasks WHERE project_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM project_catalogs WHERE project_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM standard_outputs WHERE project_uuid = ?1", [uuid])?;
    let deleted = transaction.execute("DELETE FROM evaluation_projects WHERE uuid = ?1", [uuid])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes task `uuid` with its scheme, in one transaction; gives whether it
/// existed.
///
/// Called by: `task_list_remove` (`evaluation_plan.rs`).
pub fn task(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM task_connections WHERE task_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM task_blocks WHERE task_uuid = ?1", [uuid])?;
    let deleted = transaction.execute("DELETE FROM evaluation_tasks WHERE uuid = ?1", [uuid])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes block `uuid` with its connections, in one transaction; gives
/// whether it existed.
///
/// Called by: `task_scheme_remove_block` (`evaluation_plan.rs`).
pub fn block(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM task_connections WHERE from_block = ?1 OR to_block = ?1", [uuid])?;
    let deleted = transaction.execute("DELETE FROM task_blocks WHERE uuid = ?1", [uuid])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes the connection of block `from` to block `to`; gives whether it
/// existed.
///
/// Called by: `task_scheme_disconnect` (`evaluation_plan.rs`).
pub fn connection(db: &UserDb, from: &str, to: &str) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM task_connections WHERE from_block = ?1 AND to_block = ?2", [from, to])? > 0)
}

/// Deletes standard output `uuid` from the template of its project (the
/// tasks keep their copies); gives whether it existed.
///
/// Called by: `standard_outputs_delete` (`evaluation_plan.rs`).
pub fn standard_output(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM standard_outputs WHERE uuid = ?1", [uuid])? > 0)
}

#[cfg(test)]
mod tests {
    //! Tests of deleting projects and tasks.

    use super::*;

    use crate::db::evaluation_project::{create, read};
    use crate::model::evaluation_project::{EvaluationTask, ProjectFields};
    use crate::model::task_scheme::{BlockFields, StandardOutput, new_scheme};
    use crate::test_support::standards::database;

    #[test]
    fn a_project_goes_with_its_tasks_and_a_task_alone() {
        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned()]).unwrap();
        for uuid in ["t1", "t2"] {
            let task =
                EvaluationTask { uuid: uuid.to_owned(), catalog_uuid: "c".to_owned(), control_id: uuid.to_owned() };
            create::task(&test.db, "p", &task, &new_scheme(&format!("b-{}", task.uuid), &[]).unwrap()).unwrap();
        }

        assert!(task(&test.db, "t1").unwrap());
        assert_eq!(read::tasks(&test.db, "p").unwrap().len(), 1);

        assert!(project(&test.db, "p").unwrap());
        assert!(!project(&test.db, "p").unwrap());
        assert!(read::tasks(&test.db, "p").unwrap().is_empty());
        assert!(!read::catalog_in_use(&test.db, "c").unwrap());
    }

    #[test]
    fn a_standard_output_goes_alone_or_with_its_project() {
        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned()]).unwrap();
        for uuid in ["s1", "s2"] {
            let output = StandardOutput { uuid: uuid.to_owned(), fields: BlockFields::empty() };
            create::standard_output(&test.db, "p", &output).unwrap();
        }

        assert!(standard_output(&test.db, "s1").unwrap());
        assert!(!standard_output(&test.db, "s1").unwrap());
        assert_eq!(read::standard_outputs(&test.db, "p").unwrap().len(), 1);

        project(&test.db, "p").unwrap();
        assert!(read::standard_outputs(&test.db, "p").unwrap().is_empty());
    }

    #[test]
    fn a_scheme_has_its_requirement_and_goes_with_its_blocks() {
        use crate::model::task_scheme::{Block, BlockFields, BlockKind};

        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned()]).unwrap();
        let task_one =
            EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &task_one, &new_scheme("requirement", &[]).unwrap()).unwrap();
        let input =
            Block { uuid: "in".to_owned(), kind: BlockKind::Input, x: -200, y: 0, fields: BlockFields::empty() };
        create::block(&test.db, "t", &input).unwrap();
        create::connection(&test.db, "t", "in", "requirement").unwrap();
        create::connection(&test.db, "t", "in", "requirement").unwrap();

        let blocks = read::blocks(&test.db, "t").unwrap();
        assert_eq!((blocks[0].kind, blocks[1].kind), (BlockKind::Requirement, BlockKind::Input));
        assert_eq!(read::connections(&test.db, "t").unwrap().len(), 1);
        assert_eq!(read::block(&test.db, "in").unwrap().unwrap().0, "t");

        assert!(block(&test.db, "in").unwrap());
        assert!(read::connections(&test.db, "t").unwrap().is_empty());

        assert!(task(&test.db, "t").unwrap());
        assert!(read::blocks(&test.db, "t").unwrap().is_empty());
    }
}
