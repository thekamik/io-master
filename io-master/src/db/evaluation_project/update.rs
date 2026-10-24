//! Changing master projects: their fields and catalogs, the order of their
//! tasks.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_project::{MAX_CATALOGS, ProjectFields};
use crate::model::task_scheme::BlockFields;
use crate::model::time::now;
use crate::model::work_package::PackageFields;

use super::create::insert_package_tasks;

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

/// Replaces the fields of block `uuid`.
///
/// Called by: `task_scheme_save_block` (`evaluation_plan.rs`).
pub fn block_fields(db: &UserDb, uuid: &str, fields: &BlockFields) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE task_blocks SET name = :name, evidence = :evidence, instructions = :instructions, target = :target,
                                format = :format, condition = :condition
         WHERE uuid = :uuid",
        named_params! {
            ":uuid": uuid,
            ":name": fields.name,
            ":evidence": fields.evidence,
            ":instructions": fields.instructions,
            ":target": fields.target,
            ":format": fields.format.name(),
            ":condition": fields.condition.name(),
        },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "task_blocks", id: 0 }) }
}

/// Replaces the fields of standard output `uuid` (its `evidence` is not
/// kept: an output has none).
///
/// Called by: `standard_outputs_save` (`evaluation_plan.rs`).
pub fn standard_output(db: &UserDb, uuid: &str, fields: &BlockFields) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE standard_outputs SET name = :name, instructions = :instructions, target = :target,
                                     format = :format, condition = :condition
         WHERE uuid = :uuid",
        named_params! {
            ":uuid": uuid,
            ":name": fields.name,
            ":instructions": fields.instructions,
            ":target": fields.target,
            ":format": fields.format.name(),
            ":condition": fields.condition.name(),
        },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "standard_outputs", id: 0 }) }
}

/// Accepts proposed block `uuid`: it becomes an ordinary block of its
/// scheme. Gives whether it was a proposed block.
///
/// Called by: `task_scheme_accept` (`evaluation_plan.rs`).
pub fn accept_block(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let changed = db
        .connection()
        .execute("UPDATE task_blocks SET proposal = NULL WHERE uuid = ?1 AND proposal IS NOT NULL", [uuid])?;

    Ok(changed == 1)
}

/// Accepts the correction of block `uuid`: its fields become `fields` (the
/// correction applied, `Correction::applied`) and the correction is
/// removed, in one transaction.
///
/// Called by: `task_scheme_accept` (`evaluation_plan.rs`).
pub fn accept_correction(db: &UserDb, uuid: &str, fields: &BlockFields) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    write_fields(&transaction, uuid, fields)?;
    transaction.execute("DELETE FROM block_corrections WHERE block_uuid = ?1", [uuid])?;
    transaction.commit()?;

    Ok(())
}

/// Accepts every proposal of task `task`: the proposed blocks become
/// ordinary blocks, and every block of `corrected` gets its fields (its
/// correction applied), in one transaction; the corrections are removed.
///
/// Called by: `task_scheme_accept_all` (`evaluation_plan.rs`).
pub fn accept_all(db: &UserDb, task: &str, corrected: &[(String, BlockFields)]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("UPDATE task_blocks SET proposal = NULL WHERE task_uuid = ?1", [task])?;

    for (uuid, fields) in corrected {
        write_fields(&transaction, uuid, fields)?;
    }

    transaction.execute("DELETE FROM block_corrections WHERE task_uuid = ?1", [task])?;
    transaction.commit()?;

    Ok(())
}

/// Writes the fields of block `uuid` in the transaction of the caller.
///
/// Called by: [`accept_correction`], [`accept_all`].
fn write_fields(transaction: &rusqlite::Transaction<'_>, uuid: &str, fields: &BlockFields) -> Result<(), DbError> {
    let changed = transaction.execute(
        "UPDATE task_blocks SET name = :name, evidence = :evidence, instructions = :instructions WHERE uuid = :uuid",
        named_params! {
            ":uuid": uuid,
            ":name": fields.name,
            ":evidence": fields.evidence,
            ":instructions": fields.instructions,
        },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "task_blocks", id: 0 }) }
}

/// Accepts the related element `control` of catalog `catalog` that the AI
/// proposed for task `task`; gives whether it was proposed.
///
/// Called by: `task_links_accept` (`evaluation_plan.rs`).
pub fn accept_link(db: &UserDb, task: &str, catalog: &str, control: &str) -> Result<bool, DbError> {
    let changed = db.connection().execute(
        "UPDATE task_links SET proposed = 0
         WHERE task_uuid = ?1 AND catalog_uuid = ?2 AND control_id = ?3 AND proposed = 1",
        [task, catalog, control],
    )?;

    Ok(changed == 1)
}

/// Replaces the fields and the tasks of work package `uuid`, in one
/// transaction; refused when a task is in another package.
///
/// Called by: `work_packages_save` (`evaluation_plan.rs`).
pub fn work_package(db: &UserDb, uuid: &str, fields: &PackageFields, tasks: &[String]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let changed = transaction.execute(
        "UPDATE work_packages SET name = :name, evaluator = :evaluator, note = :note, modified = :modified
         WHERE uuid = :uuid",
        named_params! {
            ":uuid": uuid,
            ":name": fields.name,
            ":evaluator": fields.evaluator,
            ":note": fields.note,
            ":modified": now(),
        },
    )?;

    if changed != 1 {
        return Err(DbError::NoRow { table: "work_packages", id: 0 });
    }

    transaction.execute("DELETE FROM package_tasks WHERE package_uuid = ?1", [uuid])?;
    insert_package_tasks(&transaction, uuid, tasks)?;
    transaction.commit()?;

    Ok(())
}

/// Places block `uuid` at `x`, `y` in the drawing (where a drag ended).
///
/// Called by: `task_scheme_move_block` (`evaluation_plan.rs`).
pub fn block_position(db: &UserDb, uuid: &str, x: i64, y: i64) -> Result<(), DbError> {
    let changed = db
        .connection()
        .execute("UPDATE task_blocks SET x = ?1, y = ?2 WHERE uuid = ?3", rusqlite::params![x, y, uuid])?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "task_blocks", id: 0 }) }
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
    use crate::model::task_scheme::new_scheme;
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
            create::task(&test.db, "p", &task, &new_scheme(&format!("b-{}", task.uuid), &[]).unwrap()).unwrap();
        }

        assert!(move_task(&test.db, "t3", true).unwrap());
        assert!(!move_task(&test.db, "t1", true).unwrap());
        // After the move t2 is the last one.
        assert!(!move_task(&test.db, "t2", false).unwrap());

        let order: Vec<String> = read::tasks(&test.db, "p").unwrap().into_iter().map(|task| task.uuid).collect();
        assert_eq!(order, ["t1", "t3", "t2"]);
    }

    #[test]
    fn the_fields_and_the_place_of_a_block_are_kept() {
        use crate::model::task_scheme::{BlockFields, OutputCondition, OutputFormat};

        let test = database();
        create::project(&test.db, "p", &fields("p"), &["c".to_owned()]).unwrap();
        let task = EvaluationTask { uuid: "t".to_owned(), catalog_uuid: "c".to_owned(), control_id: "x".to_owned() };
        create::task(&test.db, "p", &task, &new_scheme("req", &[]).unwrap()).unwrap();

        let mut written = BlockFields::empty();
        written.instructions = "Assess the interfaces.".to_owned();
        written.format = OutputFormat::Latex;
        written.condition = OutputCondition::NotMet;
        block_fields(&test.db, "req", &written).unwrap();
        block_position(&test.db, "req", 40, -12).unwrap();

        let found = read::block(&test.db, "req").unwrap().unwrap().1;
        assert_eq!((found.fields, found.x, found.y), (written, 40, -12));
        assert!(block_position(&test.db, "none", 0, 0).is_err());
    }

    #[test]
    fn the_fields_of_a_standard_output_are_kept() {
        use crate::model::task_scheme::{BlockFields, OutputCondition, OutputFormat, StandardOutput};

        let test = database();
        create::project(&test.db, "p", &fields("p"), &["c".to_owned()]).unwrap();
        create::standard_output(&test.db, "p", &StandardOutput { uuid: "s".to_owned(), fields: BlockFields::empty() })
            .unwrap();

        let mut written = BlockFields::empty();
        written.name = "pETR".to_owned();
        written.format = OutputFormat::Latex;
        written.condition = OutputCondition::Met;
        standard_output(&test.db, "s", &written).unwrap();

        assert_eq!(
            read::standard_output(&test.db, "s").unwrap().unwrap(),
            ("p".to_owned(), StandardOutput { uuid: "s".to_owned(), fields: written.clone() })
        );
        assert!(standard_output(&test.db, "none", &written).is_err());
    }
}
