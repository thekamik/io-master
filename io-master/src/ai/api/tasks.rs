//! The tasks of the AI: what a question is for. Every task is asked
//! in one role (`roles.rs`) and has its own instructions, which the
//! evaluator may change (kept in `db/ai/instructions/`).
//!
//! ```text
//! task       role            instructions
//! chat       consultation    none here: the system prompt of the connection
//! check      none            none: the check of a connection (checks.rs)
//! ```
//!
//! ```text
//! instructions()       GET    /tasks/{task}/instructions
//! set_instructions()   PUT    /tasks/{task}/instructions (empty text: DELETE)
//! ```

use crate::db::ai::instructions as db_instructions;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use crate::model::ai::Task;

/// The instructions when the evaluator has not changed them; `None`
/// for a task without instructions of its own.
pub fn default_instructions(task: Task) -> Option<&'static str> {
    match task {
        Task::Chat | Task::Check => None,
    }
}

/// The instructions of `task`: those the evaluator wrote, else the
/// default ones; `None` for a task without instructions.
pub fn instructions(db: &UserDb, task: Task) -> Result<Option<String>, DbError> {
    let Some(default) = default_instructions(task) else {
        return Ok(None);
    };

    Ok(Some(db_instructions::read::text(db, task)?.unwrap_or_else(|| default.to_owned())))
}

/// Stores the instructions of `task`; an empty text brings the default
/// ones back. Gives the instructions now in force; `None` for a task
/// without instructions (nothing is stored).
pub fn set_instructions(db: &UserDb, task: Task, text: &str) -> Result<Option<String>, DbError> {
    let Some(default) = default_instructions(task) else {
        return Ok(None);
    };

    if text.trim().is_empty() {
        db_instructions::delete::text(db, task)?;
        Ok(Some(default.to_owned()))
    } else {
        db_instructions::update::text(db, task, text)?;
        Ok(Some(text.to_owned()))
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tests;
