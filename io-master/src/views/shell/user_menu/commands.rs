//! The command of the user menu.

use tauri::State;

use crate::tauri_ui::state::UiState;

/// Logs out: ends the session of the evaluator.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn user_menu_log_out(state: State<'_, UiState>) {
    state.log_out();
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
