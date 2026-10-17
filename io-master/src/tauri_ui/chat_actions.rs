//! The chats: the list, an open chat, deleting, and a message with
//! its reply shown while it is written.
//!
//! A reply runs while the command [`send_message`] waits for it; its
//! pieces go to the page through `events` as they arrive. "Stop"
//! (the command `stop_reply`, through [`UiState::stop_reply`]) stops
//! the reply (`ai/api/completions.rs`); what was written up to then is
//! stored, like every reply, with how it ended (`ai/chat/chats.rs`),
//! and the question goes into the journal of the AI.

use crate::ai::api::completions;
use crate::ai::api::tasks::Task;
use crate::ai::chat::chats;
use crate::ai::chat::conversation::{self, Prepared};

use super::error::UiError;
use super::serializer::chat_view::{self, ChatSummaryView, ChatView, MessageView, ReplyEvent};
use super::state::UiState;
use super::user_data::with_user_db;

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
/// waits for the reply, passing its events to `events` while it is
/// written. `events` returns `false` when the page can no longer be
/// reached; the reply is then stopped as if "Stop" was pressed.
pub async fn send_message(
    state: &UiState,
    chat_id: Option<i64>,
    text: String,
    events: impl Fn(ReplyEvent) -> bool + Send,
) -> Result<(), UiError> {
    let Prepared { chat_id, title, model, turns } =
        with_user_db(state, move |db| Ok(conversation::prepare(db, chat_id, &text)?)).await?;

    events(ReplyEvent::Chat { id: chat_id, title });

    // The pieces go to the page as they arrive; a page that is gone
    // stops the reply, as "Stop" does.
    let stop = state.start_reply();
    let on_piece = |text: &str| events(ReplyEvent::Text { text: text.to_owned() });
    let reply = completions::create(&model, &turns, &stop, on_piece).await;

    let (ending, note) = chats::ending_of(&reply);
    let stored = with_user_db(state, move |db| {
        chats::add_reply(db, chat_id, &reply)?;
        completions::record(db, &model, Task::Chat, &turns, &reply)?;
        Ok(())
    })
    .await;

    let mut notice = chat_view::ending_notice(&ending, note.as_deref());

    if let Err(error) = stored {
        let text = format!("The reply was not stored: {error}.");
        notice = Some(notice.map(|before| format!("{before} {text}")).unwrap_or(text));
    }

    events(ReplyEvent::End { ending: ending.name(), notice });

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chat_actions_tests.rs"]
mod tests;
