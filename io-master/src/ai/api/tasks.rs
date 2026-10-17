//! The tasks of the AI: what a question is for. Every task is asked
//! in one role (`roles.rs`) and has its own instructions, which the
//! evaluator may change (kept in `db/ai/instructions/`).
//!
//! ```text
//! task       role            instructions
//! chapter    text_analysis   test_projects/analysis/chapter.rs (the analysis of a chapter)
//! contents   text_analysis   test_projects/analysis/contents.rs (the table of contents)
//! chat       consultation    none here: the system prompt of the connection
//! check      none            none: the check of a connection (checks.rs)
//! ```
//!
//! ```text
//! instructions()       GET    /tasks/{task}/instructions
//! set_instructions()   PUT    /tasks/{task}/instructions (empty text: DELETE)
//! ```

use crate::db::ai::instructions::{delete as remove, read, update};
use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::test_projects::analysis::{chapter as analysis, contents};

use super::roles::Role;

/// A task of the AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Task {
    /// The analysis of a chapter of a standard: tiles and links.
    Chapter,
    /// Reading the table of contents of a standard: the schemes.
    Contents,
    /// A message of the chat.
    Chat,
    /// The check of a connection.
    Check,
}

impl Task {
    pub const ALL: [Task; 4] = [Task::Chapter, Task::Contents, Task::Chat, Task::Check];

    /// The name kept in the database and in the journal.
    pub fn name(self) -> &'static str {
        match self {
            Task::Chapter => "chapter",
            Task::Contents => "contents",
            Task::Chat => "chat",
            Task::Check => "check",
        }
    }

    pub fn from_name(name: &str) -> Option<Task> {
        Task::ALL.into_iter().find(|task| task.name() == name)
    }

    /// The role it is asked in; `None` for the check of a connection.
    pub fn role(self) -> Option<Role> {
        match self {
            Task::Chapter | Task::Contents => Some(Role::TextAnalysis),
            Task::Chat => Some(Role::Consultation),
            Task::Check => None,
        }
    }

    /// The instructions when the evaluator has not changed them; `None`
    /// for a task without instructions of its own.
    pub fn default_instructions(self) -> Option<&'static str> {
        match self {
            Task::Chapter => Some(analysis::DEFAULT_INSTRUCTIONS),
            Task::Contents => Some(contents::DEFAULT_INSTRUCTIONS),
            Task::Chat | Task::Check => None,
        }
    }
}

/// The instructions of `task`: those the evaluator wrote, else the
/// default ones; `None` for a task without instructions.
pub fn instructions(db: &UserDb, task: Task) -> Result<Option<String>, DbError> {
    let Some(default) = task.default_instructions() else {
        return Ok(None);
    };

    Ok(Some(read::text(db, task)?.unwrap_or_else(|| default.to_owned())))
}

/// Stores the instructions of `task`; an empty text brings the default
/// ones back. Gives the instructions now in force; `None` for a task
/// without instructions (nothing is stored).
pub fn set_instructions(db: &UserDb, task: Task, text: &str) -> Result<Option<String>, DbError> {
    let Some(default) = task.default_instructions() else {
        return Ok(None);
    };

    if text.trim().is_empty() {
        remove::text(db, task)?;
        Ok(Some(default.to_owned()))
    } else {
        update::text(db, task, text)?;
        Ok(Some(text.to_owned()))
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tests;
