//! The error of the panel of the chat: a chat or a message refused, the
//! knowledge of the agent that could not be read, a failure of the
//! database. The commands of the panel tell its kinds apart for the page.

use std::fmt;

use crate::db::error::DbError;
use crate::panels::logic::agents::error::AgentError;

/// Why a chat or a message of it was refused or failed.
#[derive(Debug)]
pub enum ChatError {
    /// The chat or the message does not hold together; holds the reason (a
    /// chat without a title, an empty message, a chat without an agent).
    Refused(String),
    /// The knowledge of the agent could not be read.
    Knowledge(AgentError),
    /// A database failed, or refused what was given to it.
    Db(DbError),
}

impl fmt::Display for ChatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChatError::Refused(reason) => formatter.write_str(reason),
            ChatError::Knowledge(error) => write!(formatter, "the knowledge of the agent: {error}"),
            ChatError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ChatError {}

impl From<AgentError> for ChatError {
    fn from(error: AgentError) -> ChatError {
        ChatError::Knowledge(error)
    }
}

impl From<DbError> for ChatError {
    fn from(error: DbError) -> ChatError {
        ChatError::Db(error)
    }
}
