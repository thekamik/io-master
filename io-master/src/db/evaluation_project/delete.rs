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
    transaction.execute(
        "DELETE FROM block_corrections WHERE task_uuid IN (SELECT uuid FROM evaluation_tasks WHERE project_uuid = ?1)",
        [uuid],
    )?;
    transaction.execute(
        "DELETE FROM task_links WHERE task_uuid IN (SELECT uuid FROM evaluation_tasks WHERE project_uuid = ?1)",
        [uuid],
    )?;
    transaction.execute("DELETE FROM evaluation_tasks WHERE project_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM project_catalogs WHERE project_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM standard_outputs WHERE project_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM task_proposals WHERE project_uuid = ?1", [uuid])?;
    transaction.execute(
        "DELETE FROM package_tasks WHERE package_uuid IN (SELECT uuid FROM work_packages WHERE project_uuid = ?1)",
        [uuid],
    )?;
    transaction.execute("DELETE FROM work_packages WHERE project_uuid = ?1", [uuid])?;
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
    transaction.execute("DELETE FROM block_corrections WHERE task_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM task_links WHERE task_uuid = ?1", [uuid])?;
    transaction.execute("DELETE FROM package_tasks WHERE task_uuid = ?1", [uuid])?;
    let deleted = transaction.execute("DELETE FROM evaluation_tasks WHERE uuid = ?1", [uuid])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes block `uuid` with its connections and its correction, in one
/// transaction; gives whether it existed.
///
/// Called by: `task_scheme_remove_block`, `task_scheme_reject` (`evaluation_plan.rs`).
pub fn block(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM task_connections WHERE from_block = ?1 OR to_block = ?1", [uuid])?;
    transaction.execute("DELETE FROM block_corrections WHERE block_uuid = ?1", [uuid])?;
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

/// Rejects the correction of block `block`; gives whether there was one.
///
/// Called by: `task_scheme_reject` (`evaluation_plan.rs`).
pub fn correction(db: &UserDb, block: &str) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM block_corrections WHERE block_uuid = ?1", [block])? > 0)
}

/// Rejects every proposal of task `task`, in one transaction: the proposed
/// blocks go with their connections, the corrections are removed. Also run
/// before a new question to the AI, so that no undecided proposal stays.
///
/// Called by: `task_scheme_reject_all` (`evaluation_plan.rs`), `task_scheme/propose.rs`.
pub fn proposals(db: &UserDb, task: &str) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM task_connections WHERE task_uuid = ?1 AND (
             from_block IN (SELECT uuid FROM task_blocks WHERE task_uuid = ?1 AND proposal IS NOT NULL)
             OR to_block IN (SELECT uuid FROM task_blocks WHERE task_uuid = ?1 AND proposal IS NOT NULL))",
        [task],
    )?;
    transaction.execute("DELETE FROM task_blocks WHERE task_uuid = ?1 AND proposal IS NOT NULL", [task])?;
    transaction.execute("DELETE FROM block_corrections WHERE task_uuid = ?1", [task])?;
    transaction.commit()?;

    Ok(())
}

/// Rejects the proposal of requirement `control` of catalog `catalog` as a
/// task of project `project`; gives whether there was one.
///
/// Called by: `task_proposals_reject` (`evaluation_plan.rs`).
pub fn task_proposal(db: &UserDb, project: &str, catalog: &str, control: &str) -> Result<bool, DbError> {
    let deleted = db.connection().execute(
        "DELETE FROM task_proposals WHERE project_uuid = ?1 AND catalog_uuid = ?2 AND control_id = ?3",
        [project, catalog, control],
    )?;

    Ok(deleted > 0)
}

/// Rejects the proposals of tasks of project `project`: those of catalog
/// `catalog`, or all when it is `None`.
///
/// Called by: `task_proposals_reject_all` (`evaluation_plan.rs`), `task_proposals/propose.rs`
/// (before a new question for a catalog).
pub fn task_proposals(db: &UserDb, project: &str, catalog: Option<&str>) -> Result<(), DbError> {
    match catalog {
        Some(catalog) => db
            .connection()
            .execute("DELETE FROM task_proposals WHERE project_uuid = ?1 AND catalog_uuid = ?2", [project, catalog])?,
        None => db.connection().execute("DELETE FROM task_proposals WHERE project_uuid = ?1", [project])?,
    };

    Ok(())
}

/// Removes the related element `control` of catalog `catalog` from task
/// `task`: a proposal rejected, or an accepted element removed; gives
/// whether there was one.
///
/// Called by: `task_links_remove` (`evaluation_plan.rs`).
pub fn task_link(db: &UserDb, task: &str, catalog: &str, control: &str) -> Result<bool, DbError> {
    let deleted = db.connection().execute(
        "DELETE FROM task_links WHERE task_uuid = ?1 AND catalog_uuid = ?2 AND control_id = ?3",
        [task, catalog, control],
    )?;

    Ok(deleted > 0)
}

/// Rejects the related elements of task `task` that are only proposed;
/// run before a new question to the AI.
///
/// Called by: `task_links/propose.rs`.
pub fn proposed_links(db: &UserDb, task: &str) -> Result<(), DbError> {
    db.connection().execute("DELETE FROM task_links WHERE task_uuid = ?1 AND proposed = 1", [task])?;

    Ok(())
}

/// Deletes work package `uuid`; its tasks stay in the master project, free
/// for another package. Gives whether it existed.
///
/// Called by: `work_packages_delete` (`evaluation_plan.rs`).
pub fn work_package(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM package_tasks WHERE package_uuid = ?1", [uuid])?;
    let deleted = transaction.execute("DELETE FROM work_packages WHERE uuid = ?1", [uuid])? > 0;
    transaction.commit()?;

    Ok(deleted)
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
        let input = Block {
            uuid: "in".to_owned(),
            kind: BlockKind::Input,
            x: -200,
            y: 0,
            fields: BlockFields::empty(),
            proposal: None,
        };
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

    #[test]
    fn the_proposals_of_a_task_are_accepted_or_rejected() {
        use crate::db::evaluation_project::update;
        use crate::model::task_scheme::{Block, BlockKind, Correction};

        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned()]).unwrap();
        let one = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &one, &new_scheme("r", &[]).unwrap()).unwrap();
        for uuid in ["p1", "p2"] {
            let proposed = Block {
                uuid: uuid.to_owned(),
                kind: BlockKind::Input,
                x: -320,
                y: 0,
                fields: BlockFields::empty(),
                proposal: Some("The requirement asks for it.".to_owned()),
            };
            create::block(&test.db, "t", &proposed).unwrap();
            create::connection(&test.db, "t", uuid, "r").unwrap();
        }
        let correction = Correction {
            block: "r".to_owned(),
            name: String::new(),
            evidence: String::new(),
            instructions: "Check the roles.".to_owned(),
            reason: "Clearer.".to_owned(),
        };
        create::correction(&test.db, "t", &correction).unwrap();

        assert!(update::accept_block(&test.db, "p1").unwrap());
        assert!(!update::accept_block(&test.db, "p1").unwrap(), "accepted once");
        assert_eq!(read::correction(&test.db, "r").unwrap(), Some(correction.clone()));

        proposals(&test.db, "t").unwrap();

        let left: Vec<String> = read::blocks(&test.db, "t").unwrap().into_iter().map(|block| block.uuid).collect();
        assert_eq!(left, ["r", "p1"]);
        assert_eq!(read::connections(&test.db, "t").unwrap().len(), 1);
        assert!(read::corrections(&test.db, "t").unwrap().is_empty());

        create::correction(&test.db, "t", &correction).unwrap();
        update::accept_correction(&test.db, "r", &correction.applied(&BlockFields::empty())).unwrap();
        assert_eq!(read::block(&test.db, "r").unwrap().unwrap().1.fields.instructions, "Check the roles.");
        assert!(!self::correction(&test.db, "r").unwrap());
    }

    #[test]
    fn a_related_element_is_accepted_removed_or_rejected_with_the_others_proposed() {
        use crate::db::evaluation_project::update;
        use crate::model::evaluation_project::TaskLink;

        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned(), "cem".to_owned()]).unwrap();
        let one = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &one, &new_scheme("r", &[]).unwrap()).unwrap();
        for unit in ["u1", "u2", "u3"] {
            let link = TaskLink {
                catalog_uuid: "cem".to_owned(),
                control_id: unit.to_owned(),
                reason: "Its work unit.".to_owned(),
                proposed: true,
            };
            create::task_link(&test.db, "t", &link).unwrap();
        }

        assert!(update::accept_link(&test.db, "t", "cem", "u1").unwrap());
        assert!(!update::accept_link(&test.db, "t", "cem", "u1").unwrap());
        assert!(task_link(&test.db, "t", "cem", "u2").unwrap());
        proposed_links(&test.db, "t").unwrap();

        let left = read::task_links(&test.db, "t").unwrap();
        assert_eq!((left.len(), left[0].control_id.as_str(), left[0].proposed), (1, "u1", false));
        task(&test.db, "t").unwrap();
        assert!(read::task_links(&test.db, "t").unwrap().is_empty());
    }

    #[test]
    fn a_work_package_keeps_a_task_once_and_frees_it_when_deleted() {
        use crate::db::evaluation_project::update;
        use crate::model::work_package::{PackageFields, WorkPackage};

        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned()]).unwrap();
        for uuid in ["t1", "t2", "t3"] {
            let made =
                EvaluationTask { uuid: uuid.to_owned(), catalog_uuid: "c".to_owned(), control_id: uuid.to_owned() };
            create::task(&test.db, "p", &made, &new_scheme(&format!("r-{uuid}"), &[]).unwrap()).unwrap();
        }
        let package_fields =
            PackageFields { name: "ADV".to_owned(), evaluator: "Anna".to_owned(), note: String::new() };
        let package = WorkPackage {
            uuid: "w".to_owned(),
            fields: package_fields.clone(),
            tasks: vec!["t2".to_owned(), "t1".to_owned()],
        };
        create::work_package(&test.db, "p", &package).unwrap();

        assert_eq!(read::work_package(&test.db, "w").unwrap().unwrap(), ("p".to_owned(), package.clone()));
        assert_eq!(read::package_of_task(&test.db, "t1").unwrap().as_deref(), Some("ADV"));
        let twice = WorkPackage { uuid: "x".to_owned(), fields: package_fields.clone(), tasks: vec!["t1".to_owned()] };
        assert!(create::work_package(&test.db, "p", &twice).is_err(), "a task is in one package at most");

        update::work_package(&test.db, "w", &package_fields, &["t3".to_owned()]).unwrap();
        assert_eq!(read::work_packages(&test.db, "p").unwrap()[0].tasks, ["t3"]);
        task(&test.db, "t3").unwrap();
        assert!(read::work_packages(&test.db, "p").unwrap()[0].tasks.is_empty());

        assert!(work_package(&test.db, "w").unwrap());
        assert!(read::work_packages(&test.db, "p").unwrap().is_empty());
    }

    #[test]
    fn a_proposed_task_is_rejected_or_decided_by_the_task_made() {
        use crate::model::evaluation_project::TaskProposal;

        let test = database();
        let fields = ProjectFields { title: "p".to_owned(), description: String::new() };
        create::project(&test.db, "p", &fields, &["c".to_owned(), "d".to_owned()]).unwrap();
        for (catalog, control) in [("c", "x"), ("c", "y"), ("d", "z")] {
            let proposal = TaskProposal {
                catalog_uuid: catalog.to_owned(),
                control_id: control.to_owned(),
                reason: "Why.".to_owned(),
            };
            create::task_proposal(&test.db, "p", &proposal).unwrap();
        }

        let made = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &made, &new_scheme("r", &[]).unwrap()).unwrap();
        assert_eq!(read::task_proposals(&test.db, "p").unwrap().len(), 2);

        assert!(task_proposal(&test.db, "p", "c", "y").unwrap());
        assert!(!task_proposal(&test.db, "p", "c", "y").unwrap());
        task_proposals(&test.db, "p", Some("c")).unwrap();
        assert_eq!(read::task_proposals(&test.db, "p").unwrap()[0].control_id, "z");
        task_proposals(&test.db, "p", None).unwrap();
        assert!(read::task_proposals(&test.db, "p").unwrap().is_empty());
    }
}
