//! The command of the list of chats.

use tauri::State;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::ai::chats as db_chats;

use super::view::ChatSummaryView;

/// The chats of the evaluator, the newest first.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn chat_list_get(state: State<'_, UiState>) -> Result<Vec<ChatSummaryView>, UiError> {
    let session = state.session()?;
    let chats = db_chats::read::chats(&session.db())?;

    Ok(ChatSummaryView::all(chats))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
