//! The chats of an evaluator with an AI model, kept in their
//! encrypted database (`db/user_db.rs`, tables `chats` and
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
//! into the local time of the computer when they are read.

use rusqlite::{OptionalExtension, params};

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

use super::client::{Reply, ReplyEnd};
use super::request::{Speaker, Turn};
use super::response::StopReason;

/// The longest title of a chat, in characters; the title is the
/// beginning of the first message.
const TITLE_LENGTH: usize = 60;

/// The current time in SQL, as seconds since 1970.
const NOW: &str = "CAST(strftime('%s', 'now') AS INTEGER)";

/// The local time of a column of seconds, as "2026-10-14 15:04".
fn local_time(column: &str) -> String {
    format!("strftime('%Y-%m-%d %H:%M', {column}, 'unixepoch', 'localtime')")
}

/// A chat in the list of chats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSummary {
    pub id: i64,
    pub title: String,
    /// Local time it was started: "2026-10-14 15:04".
    pub created: String,
}

/// How a reply of the model ended (see the top of this file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    Complete,
    Cut,
    Stopped,
    Refused,
    Failed,
}

impl Ending {
    /// The name kept in the database, also used by the chat page.
    pub fn name(&self) -> &'static str {
        match self {
            Ending::Complete => "complete",
            Ending::Cut => "cut",
            Ending::Stopped => "stopped",
            Ending::Refused => "refused",
            Ending::Failed => "failed",
        }
    }

    /// The ending from its name; an unknown name counts as a
    /// complete reply.
    fn from_name(name: &str) -> Ending {
        match name {
            "cut" => Ending::Cut,
            "stopped" => Ending::Stopped,
            "refused" => Ending::Refused,
            "failed" => Ending::Failed,
            _ => Ending::Complete,
        }
    }
}

/// A message of a chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub speaker: Speaker,
    pub text: String,
    /// Local time it was written: "2026-10-14 15:04".
    pub created: String,
    /// For a reply: the model that wrote it.
    pub model: Option<String>,
    /// For a reply: how it ended. `Complete` for the messages of
    /// the evaluator.
    pub ending: Ending,
    /// For a failed reply: the explanation.
    pub note: Option<String>,
}

/// The name of a speaker in the database.
fn speaker_name(speaker: Speaker) -> &'static str {
    match speaker {
        Speaker::Evaluator => "evaluator",
        Speaker::Model => "model",
    }
}

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
    let connection = db.connection();

    connection.execute(
        &format!("INSERT INTO chats (title, created) VALUES (?1, {NOW})"),
        [title_of(first_message)],
    )?;

    let id = connection.last_insert_rowid();
    add_message(db, id, first_message)?;

    Ok(id)
}

/// The chats, the newest first.
pub fn list(db: &UserDb) -> Result<Vec<ChatSummary>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT id, title, {} FROM chats ORDER BY created DESC, id DESC",
        local_time("created")
    ))?;

    let chats = statement
        .query_map([], |row| {
            Ok(ChatSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                created: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(chats)
}

/// The title of a chat, or `None` when there is no such chat.
pub fn title(db: &UserDb, chat_id: i64) -> Result<Option<String>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT title FROM chats WHERE id = ?1", [chat_id], |row| row.get(0))
        .optional()?)
}

/// The messages of a chat, in order.
pub fn messages(db: &UserDb, chat_id: i64) -> Result<Vec<ChatMessage>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT speaker, text, {}, model, ending, note FROM messages WHERE chat_id = ?1 ORDER BY id",
        local_time("created")
    ))?;

    let messages = statement
        .query_map([chat_id], |row| {
            let speaker: String = row.get(0)?;
            let ending: Option<String> = row.get(4)?;

            Ok(ChatMessage {
                speaker: if speaker == "model" { Speaker::Model } else { Speaker::Evaluator },
                text: row.get(1)?,
                created: row.get(2)?,
                model: row.get(3)?,
                ending: ending.as_deref().map(Ending::from_name).unwrap_or(Ending::Complete),
                note: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}

/// Adds a message of the evaluator to a chat.
pub fn add_message(db: &UserDb, chat_id: i64, text: &str) -> Result<(), DbError> {
    db.connection().execute(
        &format!("INSERT INTO messages (chat_id, speaker, text, created) VALUES (?1, ?2, ?3, {NOW})"),
        params![chat_id, speaker_name(Speaker::Evaluator), text],
    )?;

    Ok(())
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

    db.connection().execute(
        &format!(
            "INSERT INTO messages (chat_id, speaker, text, created, model, ending, note, input_tokens, output_tokens)
             VALUES (?1, ?2, ?3, {NOW}, ?4, ?5, ?6, ?7, ?8)"
        ),
        params![
            chat_id,
            speaker_name(Speaker::Model),
            text,
            reply.model,
            ending.name(),
            note,
            reply.input_tokens as i64,
            reply.output_tokens as i64,
        ],
    )?;

    Ok(())
}

/// Deletes a chat with its messages; returns whether it existed.
pub fn delete(db: &UserDb, chat_id: i64) -> Result<bool, DbError> {
    let connection = db.connection();

    // The text of the chat is overwritten in the file, not only
    // marked as free (see UserDb::remove_secret).
    connection.execute_batch("PRAGMA secure_delete = ON")?;
    connection.execute("DELETE FROM messages WHERE chat_id = ?1", [chat_id])?;

    Ok(connection.execute("DELETE FROM chats WHERE id = ?1", [chat_id])? > 0)
}

/// The conversation as it is sent to the model: every message with
/// text, except refused replies, whose text is already dropped.
pub fn turns(messages: &[ChatMessage]) -> Vec<Turn> {
    messages
        .iter()
        .filter(|message| message.ending != Ending::Refused && !message.text.is_empty())
        .map(|message| Turn {
            speaker: message.speaker,
            text: message.text.clone(),
        })
        .collect()
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "chats_tests.rs"]
mod tests;
