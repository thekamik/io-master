//! The commands of the instructions to the AI.

use tauri::State;

use crate::ai::api::tasks;
use crate::model::ai::Task;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

/// The task of the instructions `which`, one of those that build a
/// catalog.
fn task(which: &str) -> Result<Task, UiError> {
    let Some(task) = Task::from_name(which) else {
        return Err(UiError::Invalid("there are no such instructions".to_owned()));
    };

    ensure!(
        matches!(task, Task::CatalogOutline | Task::CatalogSection),
        UiError::Invalid("there are no such instructions".to_owned())
    );

    Ok(task)
}

/// The instructions `which`: those the evaluator changed, or the default
/// ones.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_instructions_get(state: State<'_, UiState>, which: String) -> Result<String, UiError> {
    let session = state.session()?;

    Ok(tasks::instructions(&session.db(), task(&which)?)?.unwrap_or_default())
}

/// Stores the instructions `which`; an empty text brings the default ones
/// back. Gives the instructions now in force.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_instructions_save(state: State<'_, UiState>, which: String, text: String) -> Result<String, UiError> {
    let session = state.session()?;

    Ok(tasks::set_instructions(&session.db(), task(&which)?, &text)?.unwrap_or_default())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
