//! The instructions of the tasks of the AI (`Instructions`), in the table
//! `ai_instructions` of the database of the user, one row per task:
//! create, read, update; never deleted (user, 2026-10-28).
//!
//! ```text
//! ai_instructions   task (one row each), text
//! ```

use rusqlite::named_params;

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::model::ai::instructions::{AiTask, Instructions};

/// The longest instructions, in characters.
pub const MAX_INSTRUCTIONS: usize = 20_000;

/// The table, made when it is missing (`database.rs`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_instructions (
        task TEXT PRIMARY KEY CHECK (task IN ('assessment', 'consultation')),
        text TEXT NOT NULL
    );
";

/// The name of a task in the database.
fn task_name(task: AiTask) -> &'static str {
    match task {
        AiTask::Assessment => "assessment",
        AiTask::Consultation => "consultation",
    }
}

/// Refuses instructions over their limit.
///
/// Called by: [`create_instructions`], [`update_instructions`].
fn check_instructions(instructions: &Instructions) -> Result<(), DbError> {
    ensure!(
        instructions.text.chars().count() <= MAX_INSTRUCTIONS,
        DbError::Inconsistent(format!("instructions over {MAX_INSTRUCTIONS} characters"))
    );
    ensure!(!instructions.text.is_empty(), DbError::Inconsistent("empty instructions".to_owned()));

    Ok(())
}

/// Stores the instructions of a task that has none.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_instructions`].
pub fn create_instructions(db: &Database, instructions: &Instructions) -> Result<(), DbError> {
    check_instructions(instructions)?;

    db.connection().execute(
        "INSERT INTO ai_instructions (task, text) VALUES (:task, :text)",
        named_params! { ":task": task_name(instructions.task), ":text": instructions.text },
    )?;

    Ok(())
}

/// Reads the instructions of the task `task`; `None` when it has none.
///
/// Called by: nothing yet (the AI layer rewritten on `db/`).
pub fn read_instructions(db: &Database, task: AiTask) -> Result<Option<Instructions>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT text FROM ai_instructions WHERE task = :task")?;
    let mut rows = statement.query(named_params! { ":task": task_name(task) })?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    Ok(Some(Instructions { task, text: row.get("text")? }))
}

/// Writes the instructions of a task anew.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_instructions`].
pub fn update_instructions(db: &Database, instructions: &Instructions) -> Result<(), DbError> {
    check_instructions(instructions)?;

    let changed = db.connection().execute(
        "UPDATE ai_instructions SET text = :text WHERE task = :task",
        named_params! { ":task": task_name(instructions.task), ":text": instructions.text },
    )?;

    ensure!(changed == 1, DbError::Inconsistent(format!("no instructions of {}", task_name(instructions.task))));

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the instructions: stored, read, changed, one per task.

    use super::*;

    use crate::test_support::database::database;

    fn assessment(text: &str) -> Instructions {
        Instructions { task: AiTask::Assessment, text: text.to_owned() }
    }

    #[test]
    fn the_instructions_of_a_task_are_stored_read_and_changed() {
        let (_directory, db) = database();

        assert!(read_instructions(&db, AiTask::Assessment).unwrap().is_none());

        create_instructions(&db, &assessment("Assess the work unit.")).unwrap();
        update_instructions(&db, &assessment("Assess the work unit; quote the ST.")).unwrap();

        assert_eq!(
            read_instructions(&db, AiTask::Assessment).unwrap(),
            Some(assessment("Assess the work unit; quote the ST."))
        );
        assert!(read_instructions(&db, AiTask::Consultation).unwrap().is_none());
    }

    #[test]
    fn a_second_text_of_a_task_and_empty_instructions_are_refused() {
        let (_directory, db) = database();

        create_instructions(&db, &assessment("One.")).unwrap();

        assert!(matches!(create_instructions(&db, &assessment("Two.")), Err(DbError::Sql(_))));
        assert!(matches!(create_instructions(&db, &assessment("")), Err(DbError::Inconsistent(_))));
        assert!(matches!(
            update_instructions(&db, &Instructions { task: AiTask::Consultation, text: "x".to_owned() }),
            Err(DbError::Inconsistent(_))
        ));
    }
}
