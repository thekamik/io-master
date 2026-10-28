//! Reading chats and their messages.

use rusqlite::OptionalExtension;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::chats::{ChatMessage, ChatSummary};

use super::rows::{local_time, message_from_row, summary_from_row};

/// The chats, the newest first.
pub fn chats(db: &UserDb) -> Result<Vec<ChatSummary>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT id, title, {} AS local_created FROM chats ORDER BY created DESC, id DESC",
        local_time("created")
    ))?;

    Ok(statement.query_map([], summary_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// The title of a chat, or `None` when there is no such chat.
pub fn chat_title(db: &UserDb, chat_id: i64) -> Result<Option<String>, DbError> {
    Ok(db.connection().query_row("SELECT title FROM chats WHERE id = ?1", [chat_id], |row| row.get(0)).optional()?)
}

/// The messages of a chat, in order.
pub fn messages(db: &UserDb, chat_id: i64) -> Result<Vec<ChatMessage>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT speaker, text, {} AS local_created, model, ending, note FROM messages WHERE chat_id = ?1 ORDER BY id",
        local_time("created")
    ))?;

    Ok(statement.query_map([chat_id], message_from_row)?.collect::<Result<Vec<_>, _>>()?)
}
