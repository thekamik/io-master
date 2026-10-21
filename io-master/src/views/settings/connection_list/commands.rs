//! The command of the list of connections.

use tauri::State;

use crate::ai::api::connections;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

use super::view::ConnectionListView;

/// The connections of the evaluator by name, and the one the chat uses.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn connection_list_get(state: State<'_, UiState>) -> Result<ConnectionListView, UiError> {
    let session = state.session()?;
    let db = session.db();

    Ok(ConnectionListView::new(connections::list(&db)?, connections::active(&db)?))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
