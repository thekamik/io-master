//! The command of the state of the program.

use tauri::State;

use crate::app::state::UiState;

use super::view::ProgramView;

/// The state of the program: whether it is installed, its role, who is
/// logged in, the pages, the warnings, the version and the folder.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn program_get(state: State<'_, UiState>) -> ProgramView {
    ProgramView::new(state.role(), state.current_user(), &state.data_dir)
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
