//! The chats: the list, an open chat, deleting, and a message with
//! its reply shown while it is written.
//!
//! A reply runs while the command [`send_message`] waits for it; its
//! pieces go to the page through `events` as they arrive. "Stop"
//! (the command `stop_reply`, through [`UiState::stop_reply`]) ends
//! the passing on of the pieces, which stops the reply
//! (`ai/client.rs`); what was written up to then is stored, like
//! every reply, with how it ended (`ai/chats.rs`).

use tokio::sync::mpsc;

use crate::ai::chats;
use crate::ai::client;
use crate::ai::conversation::{self, Prepared};

use super::error::UiError;
use super::serializer::chat_view::{self, ChatSummaryView, ChatView, MessageView, ReplyEvent};
use super::state::UiState;
use super::user_data::with_user_db;

/// How many pieces of a reply may wait for the page.
const PIECE_QUEUE: usize = 64;

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

        Ok(ChatView {
            id: chat_id,
            title,
            messages: messages.into_iter().map(MessageView::from).collect(),
        })
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
    let Prepared {
        chat_id,
        title,
        connection,
        api_key,
        turns,
    } = with_user_db(state, move |db| Ok(conversation::prepare(db, chat_id, &text)?)).await?;

    events(ReplyEvent::Chat { id: chat_id, title });

    let mut stop = state.start_reply();
    let (pieces, mut piece_receiver) = mpsc::channel::<String>(PIECE_QUEUE);

    // Asks the model; `pieces` is dropped when the reply is done,
    // which ends the passing on below.
    let asking = async move {
        let reply = client::reply(&connection, api_key.as_ref(), &turns, &pieces).await;
        drop(pieces);
        reply
    };

    // Passes the pieces on. On "Stop", or when the page is gone, it
    // ends, and dropping `piece_receiver` stops the reply.
    let passing_on = async move {
        loop {
            tokio::select! {
                piece = piece_receiver.recv() => match piece {
                    Some(text) => {
                        if !events(ReplyEvent::Text { text }) {
                            break;
                        }
                    }
                    None => break,
                },
                _ = stop.wait_for(|stopped| *stopped) => break,
            }
        }

        events
    };

    let (reply, events) = tokio::join!(asking, passing_on);

    let (ending, note) = chats::ending_of(&reply);
    let stored = with_user_db(state, move |db| Ok(chats::add_reply(db, chat_id, &reply)?)).await;

    let mut notice = chat_view::ending_notice(&ending, note.as_deref());

    if let Err(error) = stored {
        let text = format!("The reply was not stored: {error}.");
        notice = Some(notice.map(|before| format!("{before} {text}")).unwrap_or(text));
    }

    events(ReplyEvent::End {
        ending: ending.name(),
        notice,
    });

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chat_actions_tests.rs"]
mod tests;
