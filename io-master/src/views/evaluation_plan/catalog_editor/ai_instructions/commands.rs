//! The commands of the instructions to the AI.

use tauri::State;

use crate::ai::api::tasks;
use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::model::ai::Task;

/// The task of the instructions `which`, one of those that build a
/// catalog.
fn task(which: &str) -> Result<Task, AppError> {
    let Some(task) = Task::from_name(which) else {
        return Err(AppError::Invalid("there are no such instructions".to_owned()));
    };

    ensure!(
        matches!(task, Task::CatalogOutline | Task::CatalogSection),
        AppError::Invalid("there are no such instructions".to_owned())
    );

    Ok(task)
}

/// The instructions `which`: those the evaluator changed, or the default
/// ones.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_instructions_get(state: State<'_, AppState>, which: String) -> Result<String, AppError> {
    let session = state.session()?;

    Ok(tasks::instructions(&session.db(), task(&which)?)?.unwrap_or_default())
}

/// Stores the instructions `which`; an empty text brings the default ones
/// back. Gives the instructions now in force.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ai_instructions_save(state: State<'_, AppState>, which: String, text: String) -> Result<String, AppError> {
    let session = state.session()?;

    Ok(tasks::set_instructions(&session.db(), task(&which)?, &text)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `ai_instructions/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::test_support::logged_in_window;

    #[test]
    fn the_instructions_are_changed_and_the_default_comes_back() {
        let window = logged_in_window();
        let default = crate::ai::tasks::catalog_outline::DEFAULT_INSTRUCTIONS;

        assert_eq!(window.call("ai_instructions_get", json!({ "which": "catalog_outline" })).unwrap(), default);

        let saved = window.call("ai_instructions_save", json!({ "which": "catalog_outline", "text": "Read it all." }));
        assert_eq!(saved.unwrap(), "Read it all.");

        let back = window.call("ai_instructions_save", json!({ "which": "catalog_outline", "text": "" })).unwrap();
        assert_eq!(back, default);

        assert_eq!(window.call("ai_instructions_get", json!({ "which": "chat" })).unwrap_err()["kind"], "invalid");
    }
}
