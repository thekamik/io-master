//! Removing the instructions of a task: it uses the default ones again.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Task;

/// Removes the instructions the evaluator wrote for `task`.
pub fn text(db: &UserDb, task: Task) -> Result<(), DbError> {
    db.connection().execute("DELETE FROM ai_instructions WHERE task = ?1", [task.name()])?;

    Ok(())
}
