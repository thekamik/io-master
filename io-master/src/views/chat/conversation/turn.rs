//! One turn of a chat: the message of the evaluator is checked and
//! stored, the model answers piece by piece (each piece goes to the page
//! as it arrives), and the reply is stored, also when it was stopped or
//! failed:
//!
//! ```text
//! prepare(db, chat, text)       check, store the message, read the history
//! completions::ask(...)         the reply, piece by piece, and the question
//!                               in the journal of the AI (ai/api/)
//! stored::add_reply(db, ...)    store it, also when stopped or failed
//! ```
//!
//! The chat is asked in the role *consultation* (`ai/api/roles.rs`): the
//! connection given to that role, else the active one. The database is
//! held for each step only, never while the model writes.

use std::fmt;
use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::ai::api::completions;
use crate::ai::api::roles::{self, Model, ModelError};
use crate::ai::tasks::chat as memory;
use crate::app::error::UiError;
use crate::db::ai::chats as db_chats;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::installation::session::Session;
use crate::model::ai::{ConnectionError, Role, Task, Turn};

use super::stored;
use super::view::{ReplyEvent, ending_notice};

/// The longest message an evaluator can send, in characters. A longer
/// text is better given to the model as a document (a later step).
pub const MAX_MESSAGE: usize = 100_000;

/// What a reply needs, made ready by [`prepare`].
#[derive(Debug)]
pub struct Prepared {
    /// The chat the message went to: the one asked for, or a new one.
    pub chat_id: i64,
    /// The title of that chat.
    pub title: String,
    /// The connection of the role *consultation*, checked, with its key.
    pub model: Model,
    /// The whole conversation as it is sent to the model, the new
    /// message last.
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
    /// The active connection breaks a rule (a database changed by hand,
    /// or a rule of a newer version); holds its name and why.
    BrokenConnection(String, ConnectionError),
    /// The active connection uses an API key and none is stored; holds
    /// its name.
    NoKey(String),
    /// The chat asked for does not exist.
    NoSuchChat,
    /// The page can no longer be reached: nothing more is asked.
    PageGone,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for SendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendError::Empty => formatter.write_str("the message is empty"),
            SendError::TooLong => write!(formatter, "the message is longer than {MAX_MESSAGE} characters"),
            SendError::NoConnection => {
                formatter.write_str("no AI connection is chosen; set one up in Settings, tab AI")
            }
            SendError::BrokenConnection(name, error) => {
                write!(formatter, "the connection {name} cannot be used: {error}; correct it in Settings, tab AI")
            }
            SendError::NoKey(name) => write!(
                formatter,
                "the connection {name} uses an API key and none is stored; enter one in Settings, tab AI"
            ),
            SendError::NoSuchChat => formatter.write_str("the chat does not exist"),
            SendError::PageGone => formatter.write_str("the window can no longer be reached"),
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

impl From<ModelError> for SendError {
    fn from(error: ModelError) -> SendError {
        match error {
            ModelError::NoConnection => SendError::NoConnection,
            ModelError::BrokenConnection(name, error) => SendError::BrokenConnection(name, error),
            ModelError::NoKey(name) => SendError::NoKey(name),
            ModelError::Db(error) => SendError::Db(error),
        }
    }
}

impl From<SendError> for UiError {
    fn from(error: SendError) -> UiError {
        match error {
            SendError::Db(_) | SendError::PageGone => UiError::Failed(error.to_string()),
            other => UiError::Invalid(other.to_string()),
        }
    }
}

/// Checks the message, stores it (in a new chat when `chat_id` is
/// `None`) and reads what the reply needs. Nothing is stored when the
/// message is refused.
pub fn prepare(db: &UserDb, chat_id: Option<i64>, text: &str) -> Result<Prepared, SendError> {
    let text = text.trim();

    ensure!(!text.is_empty(), SendError::Empty);
    ensure!(text.chars().count() <= MAX_MESSAGE, SendError::TooLong);

    // The connection first: without a usable one nothing is stored.
    let model = roles::model(db, Role::Consultation)?;

    let chat_id = match chat_id {
        Some(id) => {
            ensure!(db_chats::read::chat_title(db, id)?.is_some(), SendError::NoSuchChat);
            db_chats::create::message(db, id, text)?;
            id
        }
        None => stored::start(db, text)?,
    };

    Ok(Prepared {
        chat_id,
        title: db_chats::read::chat_title(db, chat_id)?.unwrap_or_default(),
        model,
        turns: memory::turns(&db_chats::read::messages(db, chat_id)?),
    })
}

/// Sends `text` to chat `chat_id` (a new chat when `None`) and waits for
/// the reply, sending its events to `events` while it is written: the
/// chat first, then the pieces, then how it ended. A page that can no
/// longer be reached stops the reply as "Stop" (`stop`) does.
pub async fn send(
    session: &Session,
    chat_id: Option<i64>,
    text: &str,
    stop: &watch::Receiver<bool>,
    events: &Channel<ReplyEvent>,
) -> Result<(), SendError> {
    let Prepared { chat_id, title, model, turns } = prepare(&session.db(), chat_id, text)?;

    ensure!(events.send(ReplyEvent::Chat { id: chat_id, title }).is_ok(), SendError::PageGone);

    let model = Arc::new(model);
    let on_piece = |piece: &str| events.send(ReplyEvent::Text { text: piece.to_owned() }).is_ok();
    let asked = completions::ask(&session.db_handle(), &model, Task::Chat, &turns, stop, on_piece).await;

    let mut problems = Vec::new();

    if let Err(error) = stored::add_reply(&session.db(), chat_id, &asked.reply) {
        problems.push(format!("The reply was not stored: {error}."));
    }

    if let Err(error) = asked.journal {
        problems.push(format!("The question was not written into the journal of the AI: {error}."));
    }

    let (ending, note) = stored::ending_of(&asked.reply);
    let notice = with_problems(ending_notice(&ending, note.as_deref()), &problems);

    ensure!(events.send(ReplyEvent::End { ending: ending.name(), notice }).is_ok(), SendError::PageGone);

    Ok(())
}

/// The notice under a reply, with what could not be done afterwards.
fn with_problems(notice: Option<String>, problems: &[String]) -> Option<String> {
    if problems.is_empty() {
        return notice;
    }

    let problems = problems.join(" ");

    match notice {
        Some(before) => Some(format!("{before} {problems}")),
        None => Some(problems),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;
