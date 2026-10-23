//! The command of the list of chats.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::ai::chats as db_chats;

use super::view::ChatSummaryView;

/// The chats of the evaluator, the newest first.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn chat_list_get(state: State<'_, AppState>) -> Result<Vec<ChatSummaryView>, AppError> {
    let session = state.session()?;
    let chats = db_chats::read::chats(&session.db())?;

    Ok(ChatSummaryView::all(chats))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `chat_list/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::chat::conversation::stored;
    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn the_chats_are_listed_the_newest_first() {
        let window = logged_in_window();
        let first = stored::start(&window.session().db(), "first").unwrap();
        let second = stored::start(&window.session().db(), "second").unwrap();

        let chats = window.call("chat_list_get", json!({})).unwrap();

        assert_eq!(chats[0]["id"], second);
        assert_eq!(chats[0]["title"], "second");
        assert_eq!(chats[1]["id"], first);
    }

    #[test]
    fn the_list_needs_a_login() {
        assert_eq!(logged_out_window().call("chat_list_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }
}
