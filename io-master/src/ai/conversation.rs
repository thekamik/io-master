//! One turn of a chat, without any user interface: the message of
//! the evaluator is checked and stored, and everything the reply
//! needs is gathered.
//!
//! A front end then asks the model (`client::reply`), shows the
//! pieces of the reply while they arrive, and stores the reply with
//! `chats::add_reply`:
//!
//! ```text
//! prepare(db, chat, text)            check, store the message, read history
//! client::reply(connection, ...)     the reply, piece by piece
//! chats::add_reply(db, ...)          store it, also when stopped or failed
//! ```

use std::fmt;

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

use super::api_key::ApiKey;
use super::chats;
use super::connection::{Checked, ConnectionError};
use super::connections;
use super::request::Turn;

/// The longest message an evaluator can send, in characters. A
/// longer text is better given to the model as a document (a later
/// step).
pub const MAX_MESSAGE: usize = 100_000;

/// What a reply needs, made ready by [`prepare`].
#[derive(Debug)]
pub struct Prepared {
    /// The chat the message went to: the one asked for, or a new one.
    pub chat_id: i64,
    /// The title of that chat.
    pub title: String,
    /// The active connection, checked.
    pub connection: Checked,
    /// Its API key, read from the database; `None` when none is
    /// stored (the connection needs none).
    pub api_key: Option<ApiKey>,
    /// The whole conversation as it is sent to the model, with the
    /// new message last.
    pub turns: Vec<Turn>,
}

/// Why a message was not sent.
#[derive(Debug)]
pub enum SendError {
    /// The message is empty (or only spaces).
    Empty,
    /// The message is longer than [`MAX_MESSAGE`].
    TooLong,
    /// No connection is active.
    NoConnection,
    /// The active connection breaks a rule (a database changed by
    /// hand, or a rule of a newer version); holds its name and why.
    BrokenConnection(String, ConnectionError),
    /// The active connection uses an API key and none is stored;
    /// holds its name.
    NoKey(String),
    /// The chat asked for does not exist.
    NoSuchChat,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for SendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::Empty => formatter.write_str("the message is empty"),
            SendError::TooLong => write!(formatter, "the message is longer than {MAX_MESSAGE} characters"),
            SendError::NoConnection => formatter.write_str("no AI connection is chosen; set one up in Settings, tab AI"),
            SendError::BrokenConnection(name, error) => {
                write!(formatter, "the connection {name} cannot be used: {error}; correct it in Settings, tab AI")
            }
            SendError::NoKey(name) => {
                write!(formatter, "the connection {name} uses an API key and none is stored; enter one in Settings, tab AI")
            }
            SendError::NoSuchChat => formatter.write_str("the chat does not exist"),
            SendError::Db(error) => write!(formatter, "the message was not stored: {error}"),
        }
    }
}

impl std::error::Error for SendError {}

impl From<DbError> for SendError {
    fn from(error: DbError) -> SendError {
        SendError::Db(error)
    }
}

/// Checks the message, stores it (in a new chat when `chat_id` is
/// `None`) and reads what the reply needs. Nothing is stored when
/// the message is refused. Blocking: opens no connection, but reads
/// and writes the database.
pub fn prepare(db: &UserDb, chat_id: Option<i64>, text: &str) -> Result<Prepared, SendError> {
    let text = text.trim();

    if text.is_empty() {
        return Err(SendError::Empty);
    }

    if text.chars().count() > MAX_MESSAGE {
        return Err(SendError::TooLong);
    }

    // The connection first: without a usable one nothing is stored.
    let id = connections::active(db)?.ok_or(SendError::NoConnection)?;
    let stored = connections::get(db, id)?.ok_or(SendError::NoConnection)?;
    let name = stored.connection.name.clone();

    let connection = stored
        .connection
        .check()
        .map_err(|error| SendError::BrokenConnection(name.clone(), error))?;
    let api_key = connections::api_key(db, id)?;

    if connection.needs_key() && api_key.is_none() {
        return Err(SendError::NoKey(name));
    }

    let chat_id = match chat_id {
        Some(id) => {
            chats::title(db, id)?.ok_or(SendError::NoSuchChat)?;
            chats::add_message(db, id, text)?;
            id
        }
        None => chats::start(db, text)?,
    };

    Ok(Prepared {
        chat_id,
        title: chats::title(db, chat_id)?.unwrap_or_default(),
        connection,
        api_key,
        turns: chats::turns(&chats::messages(db, chat_id)?),
    })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "conversation_tests.rs"]
mod tests;
