//! Storing the instructions of a task (made or replaced).

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Task;

/// Stores the instructions of `task`, replacing earlier ones.
pub fn text(db: &UserDb, task: Task, text: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO ai_instructions (task, text) VALUES (?1, ?2) \
         ON CONFLICT (task) DO UPDATE SET text = excluded.text",
        params![task.name(), text],
    )?;

    Ok(())
}
