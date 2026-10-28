//! Removing the instructions of a task: it uses the default ones again.

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::ai::Task;

/// Removes the instructions the evaluator wrote for `task`.
pub fn text(db: &UserDb, task: Task) -> Result<(), DbError> {
    db.connection().execute("DELETE FROM ai_instructions WHERE task = ?1", [task.name()])?;

    Ok(())
}
