//! The tasks of the AI: what a question is for. Every task is asked
//! in one role (`roles.rs`) and has its own instructions, which the
//! evaluator may change (kept in `db/ai/instructions/`).
//!
//! ```text
//! task              role            instructions
//! catalog_outline   text_analysis   ai/tasks/catalog_outline.rs (the groups of a catalog)
//! catalog_section   text_analysis   ai/tasks/catalog_section.rs (the controls of a group)
//! chat              consultation    none here: the system prompt of the connection
//! check             none            none: the check of a connection (checks.rs)
//! ```
//!
//! ```text
//! instructions()       GET    /tasks/{task}/instructions
//! set_instructions()   PUT    /tasks/{task}/instructions (empty text: DELETE)
//! ```

use crate::ai::tasks::{catalog_outline, catalog_section};
use crate::db::ai::instructions as db_instructions;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use crate::model::ai::Task;

/// The instructions when the evaluator has not changed them; `None`
/// for a task without instructions of its own.
pub fn default_instructions(task: Task) -> Option<&'static str> {
    match task {
        Task::CatalogOutline => Some(catalog_outline::DEFAULT_INSTRUCTIONS),
        Task::CatalogSection => Some(catalog_section::DEFAULT_INSTRUCTIONS),
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

#[cfg(test)]
mod tests {
    //! Unit tests of `tasks.rs`.

    use super::*;

    use crate::model::ai::Role;

    use crate::test_support::standards::database;

    #[test]
    fn every_task_has_its_role() {
        assert_eq!(Task::CatalogOutline.role(), Some(Role::TextAnalysis));
        assert_eq!(Task::CatalogSection.role(), Some(Role::TextAnalysis));
        assert_eq!(Task::Chat.role(), Some(Role::Consultation));
        assert_eq!(Task::Check.role(), None);

        for task in Task::ALL {
            assert_eq!(Task::from_name(task.name()), Some(task));
        }
    }

    #[test]
    fn instructions_are_changed_and_brought_back() {
        let test = database();
        let default = catalog_section::DEFAULT_INSTRUCTIONS;

        assert_eq!(instructions(&test.db, Task::CatalogSection).unwrap().as_deref(), Some(default));

        assert_eq!(set_instructions(&test.db, Task::CatalogSection, "Mine.").unwrap().as_deref(), Some("Mine."));
        assert_eq!(instructions(&test.db, Task::CatalogSection).unwrap().as_deref(), Some("Mine."));
        // The other task keeps its own.
        assert_eq!(
            instructions(&test.db, Task::CatalogOutline).unwrap().as_deref(),
            Some(catalog_outline::DEFAULT_INSTRUCTIONS)
        );

        // Empty text: the default ones again.
        assert_eq!(set_instructions(&test.db, Task::CatalogSection, "  ").unwrap().as_deref(), Some(default));
        assert_eq!(instructions(&test.db, Task::CatalogSection).unwrap().as_deref(), Some(default));

        // A task without instructions stores none.
        assert_eq!(set_instructions(&test.db, Task::Chat, "x").unwrap(), None);
        assert_eq!(instructions(&test.db, Task::Chat).unwrap(), None);
    }
}
