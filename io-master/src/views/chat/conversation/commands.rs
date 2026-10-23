//! The commands of the conversation.
//!
//! A reply runs while `conversation_send` waits for it; its pieces go to
//! the page through the channel `on_event` as they arrive. "Stop"
//! (`conversation_stop`, through `UiState::stop_reply`) stops it
//! (`ai/api/completions.rs`); what was written up to then is stored, like
//! every reply, with how it ended (`turn.rs`).

use tauri::State;
use tauri::ipc::Channel;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::ai::chats as db_chats;

use super::turn;
use super::view::{ChatView, ReplyEvent};

/// Chat `chat_id` with its messages.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_open(state: State<'_, UiState>, chat_id: i64) -> Result<ChatView, UiError> {
    let session = state.session()?;
    let db = session.db();

    let Some(title) = db_chats::read::chat_title(&db, chat_id)? else {
        return Err(UiError::Invalid("the chat does not exist".to_owned()));
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
    state: State<'_, UiState>,
    chat_id: Option<i64>,
    text: String,
    on_event: Channel<ReplyEvent>,
) -> Result<(), UiError> {
    let session = state.session()?;
    let stop = state.start_reply();

    turn::send(&session, chat_id, &text, &stop, &on_event).await?;

    Ok(())
}

/// Stops the reply being written, if there is one.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_stop(state: State<'_, UiState>) {
    state.stop_reply();
}

/// Deletes chat `chat_id` with its messages; the text is overwritten in
/// the file (`db/ai/chats/delete.rs`).
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn conversation_delete(state: State<'_, UiState>, chat_id: i64) -> Result<(), UiError> {
    let session = state.session()?;

    ensure!(db_chats::delete::chat(&session.db(), chat_id)?, UiError::Invalid("the chat does not exist".to_owned()));

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
