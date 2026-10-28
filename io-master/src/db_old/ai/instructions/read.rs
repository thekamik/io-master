//! Reading the instructions of a task, as the evaluator changed them.

use rusqlite::OptionalExtension;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::ai::Task;

/// The instructions the evaluator wrote for `task`; `None` when they
/// use the default ones.
pub fn text(db: &UserDb, task: Task) -> Result<Option<String>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT text FROM ai_instructions WHERE task = ?1", [task.name()], |row| row.get(0))
        .optional()?)
}
