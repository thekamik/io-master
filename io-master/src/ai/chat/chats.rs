//! The chats of an evaluator with an AI model, kept in their
//! encrypted database (`db/ai/chats/`, tables `chats` and
//! `messages`).
//!
//! A chat is a list of messages: those of the evaluator and the
//! replies of the model. A reply is kept also when it did not end
//! normally, with how it ended ([`Ending`]), so that the page shows
//! the same history after a reload:
//!
//! ```text
//! complete   the model finished
//! cut        the reply reached the largest length and was cut
//! stopped    the evaluator stopped it; the text up to then is kept
//! refused    the model declined; any partial text is dropped (it
//!            may be the beginning of what was declined), and only
//!            the fact is kept
//! failed     the request failed; the explanation is kept, and the
//!            text written before the failure
//! ```
//!
//! Times are kept as seconds since 1970 (UTC); SQLite turns them
//! into the local time of the computer when they are read. The SQL is
//! in `db/ai/chats/`; what of the history goes to the model is
//! decided by `memory.rs`.

use crate::ai::api::completions::{Reply, ReplyEnd, StopReason};
use crate::db::ai::chats::{create, delete as remove, read};
use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::chats::{ChatMessage, ChatSummary, Ending};

/// The longest title of a chat, in characters; the title is the
/// beginning of the first message.
const TITLE_LENGTH: usize = 60;

/// The title of a new chat: the beginning of its first message, on
/// one line.
fn title_of(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if line.chars().count() <= TITLE_LENGTH {
        return line;
    }

    let mut title: String = line.chars().take(TITLE_LENGTH - 1).collect();
    title.push('…');
    title
}

/// Starts a chat with the first message of the evaluator; returns
/// the number of the chat.
pub fn start(db: &UserDb, first_message: &str) -> Result<i64, DbError> {
    let id = create::chat(db, &title_of(first_message))?;
    add_message(db, id, first_message)?;

    Ok(id)
}

/// The chats, the newest first.
pub fn list(db: &UserDb) -> Result<Vec<ChatSummary>, DbError> {
    read::chats(db)
}

/// The title of a chat, or `None` when there is no such chat.
pub fn title(db: &UserDb, chat_id: i64) -> Result<Option<String>, DbError> {
    read::chat_title(db, chat_id)
}

/// The messages of a chat, in order.
pub fn messages(db: &UserDb, chat_id: i64) -> Result<Vec<ChatMessage>, DbError> {
    read::messages(db, chat_id)
}

/// Adds a message of the evaluator to a chat.
pub fn add_message(db: &UserDb, chat_id: i64, text: &str) -> Result<(), DbError> {
    create::message(db, chat_id, text)
}

/// How a reply ended, as it is kept: the ending, and for a failed
/// reply the explanation.
pub fn ending_of(reply: &Reply) -> (Ending, Option<String>) {
    match &reply.end {
        ReplyEnd::Finished(StopReason::Cut) => (Ending::Cut, None),
        ReplyEnd::Finished(StopReason::Refused) => (Ending::Refused, None),
        ReplyEnd::Finished(StopReason::Complete) => (Ending::Complete, None),
        ReplyEnd::Cancelled => (Ending::Stopped, None),
        ReplyEnd::Failed(error) => (Ending::Failed, Some(error.to_string())),
    }
}

/// Adds a reply of the model to a chat, with how it ended.
pub fn add_reply(db: &UserDb, chat_id: i64, reply: &Reply) -> Result<(), DbError> {
    let (ending, note) = ending_of(reply);

    // A refused reply is dropped (see the top of this file).
    let text = if ending == Ending::Refused { "" } else { reply.text.as_str() };

    create::reply(
        db,
        chat_id,
        &create::Reply {
            text,
            model: reply.model.as_deref(),
            ending: &ending,
            note: note.as_deref(),
            input_tokens: reply.input_tokens,
            output_tokens: reply.output_tokens,
        },
    )
}

/// Deletes a chat with its messages; returns whether it existed.
pub fn delete(db: &UserDb, chat_id: i64) -> Result<bool, DbError> {
    // The text of the chat is overwritten in the file (db/ai/delete.rs).
    remove::chat(db, chat_id)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chats_tests.rs"]
mod tests;
