//! The chats: the list, an open chat, deleting, and a message with
//! its reply shown while it is written.
//!
//! A reply runs while the command [`send_message`] waits for it; its
//! pieces go to the page through `events` as they arrive. "Stop"
//! (the command `stop_reply`, through [`UiState::stop_reply`]) stops
//! the reply (`ai/api/completions.rs`); what was written up to then is
//! stored, like every reply, with how it ended, and the question goes
//! into the journal of the AI (`business/chats/conversation.rs`).

use crate::business::chats::conversation::{self, ChatEvent};
use crate::business::chats::history as chats;
use crate::business::work::Cancel;

use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::chat_view::{self, ChatSummaryView, ChatView, MessageView, ReplyEvent};
use crate::tauri_ui::session::with_user_db;
use crate::tauri_ui::state::UiState;

/// The chats, the newest first.
pub async fn list_chats(state: &UiState) -> Result<Vec<ChatSummaryView>, UiError> {
    let chats = with_user_db(state, |db| Ok(chats::list(db)?)).await?;

    Ok(chats.into_iter().map(ChatSummaryView::from).collect())
}

/// A chat with its messages.
pub async fn open_chat(state: &UiState, chat_id: i64) -> Result<ChatView, UiError> {
    with_user_db(state, move |db| {
        let title = chats::title(db, chat_id)?.ok_or_else(|| UiError::Invalid("the chat does not exist".to_owned()))?;
        let messages = chats::messages(db, chat_id)?;

        Ok(ChatView { id: chat_id, title, messages: messages.into_iter().map(MessageView::from).collect() })
    })
    .await
}

/// Deletes a chat with its messages.
pub async fn delete_chat(state: &UiState, chat_id: i64) -> Result<(), UiError> {
    let deleted = with_user_db(state, move |db| Ok(chats::delete(db, chat_id)?)).await?;

    if deleted { Ok(()) } else { Err(UiError::Invalid("the chat does not exist".to_owned())) }
}

/// Sends a message (to a new chat when `chat_id` is `None`) and
/// waits for the reply (`business/chats/conversation.rs`), passing its
/// events to `events` while it is written. `events` returns `false`
/// when the page can no longer be reached; the reply is then stopped as
/// if "Stop" was pressed.
pub async fn send_message(
    state: &UiState,
    chat_id: Option<i64>,
    text: String,
    events: impl Fn(ReplyEvent) -> bool + Send + Sync,
) -> Result<(), UiError> {
    let session = state.session()?;
    let cancel = Cancel::new(state.start_reply());
    let on_event = |event: ChatEvent| {
        events(match event {
            ChatEvent::Started { chat_id, title } => ReplyEvent::Chat { id: chat_id, title },
            ChatEvent::Text(text) => ReplyEvent::Text { text },
        })
    };

    let sent = conversation::send(&session, chat_id, text, cancel, on_event).await?;

    let mut notice = chat_view::ending_notice(&sent.ending, sent.note.as_deref());

    if let Some(problem) = sent.problem {
        notice = Some(notice.map(|before| format!("{before} {problem}")).unwrap_or(problem));
    }

    events(ReplyEvent::End { ending: sent.ending.name(), notice });

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chats_tests.rs"]
mod tests;
