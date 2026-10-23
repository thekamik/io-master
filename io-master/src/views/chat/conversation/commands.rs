//! The commands of the conversation.
//!
//! A reply runs while `conversation_send` waits for it; its pieces go to
//! the page through the channel `on_event` as they arrive. "Stop"
//! (`conversation_stop`, through `AppState::stop_reply`) stops it
//! (`ai/api/completions.rs`); what was written up to then is stored, like
//! every reply, with how it ended (`turn.rs`).

use tauri::State;
use tauri::ipc::Channel;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::ai::chats as db_chats;

use super::turn;
use super::view::{ChatView, ReplyEvent};

/// Chat `chat_id` with its messages.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_open(state: State<'_, AppState>, chat_id: i64) -> Result<ChatView, AppError> {
    let session = state.session()?;
    let db = session.db();

    let Some(title) = db_chats::read::chat_title(&db, chat_id)? else {
        return Err(AppError::Invalid("the chat does not exist".to_owned()));
    };

    Ok(ChatView::new(chat_id, title, db_chats::read::messages(&db, chat_id)?))
}

/// Sends `text` to chat `chat_id` (a new chat when `None`) and waits for
/// the reply; its events go to `on_event` while it is written
/// (`turn.rs`). One reply at a time: a reply still being written is
/// stopped first.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn conversation_send(
    state: State<'_, AppState>,
    chat_id: Option<i64>,
    text: String,
    on_event: Channel<ReplyEvent>,
) -> Result<(), AppError> {
    let session = state.session()?;
    let stop = state.start_reply();

    turn::send(&session, chat_id, &text, &stop, &on_event).await?;

    Ok(())
}

/// Stops the reply being written, if there is one.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_stop(state: State<'_, AppState>) {
    state.stop_reply();
}

/// Deletes chat `chat_id` with its messages; the text is overwritten in
/// the file (`db/ai/chats/delete.rs`).
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_delete(state: State<'_, AppState>, chat_id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(db_chats::delete::chat(&session.db(), chat_id)?, AppError::Invalid("the chat does not exist".to_owned()));

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `conversation/commands.rs`, through the IPC of Tauri. A
    //! message with its reply is tested in `turn.rs`: its events go
    //! through a channel, which the mock window does not carry.

    use serde_json::json;

    use crate::views::chat::conversation::stored;
    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn a_chat_is_opened_and_deleted() {
        let window = logged_in_window();
        let id = stored::start(&window.session().db(), "What is ADV_FSP?").unwrap();

        let chat = window.call("conversation_open", json!({ "chatId": id })).unwrap();
        assert_eq!(chat["title"], "What is ADV_FSP?");
        assert_eq!(
            chat["messages"][0],
            json!({
                "speaker": "evaluator", "who": "You", "text": "What is ADV_FSP?",
                "created": chat["messages"][0]["created"], "notice": null
            })
        );

        window.call("conversation_delete", json!({ "chatId": id })).unwrap();

        assert_eq!(window.call("conversation_open", json!({ "chatId": id })).unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("conversation_delete", json!({ "chatId": id })).unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("chat_list_get", json!({})).unwrap(), json!([]));
    }

    #[test]
    fn stop_without_a_reply_does_nothing() {
        logged_in_window().call("conversation_stop", json!({})).unwrap();
    }

    #[test]
    fn the_chats_need_a_login() {
        let window = logged_out_window();

        assert_eq!(window.call("conversation_open", json!({ "chatId": 1 })).unwrap_err()["kind"], "not_logged_in");
    }
}
