//! Storing chats and their messages. What a chat keeps of a reply
//! (a refused reply drops its text) is decided by `ai/chat/chats.rs`.

use rusqlite::params;

use crate::ai::api::completions::Speaker;
use crate::ai::chat::chats::Ending;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::rows::{NOW, speaker_name};

/// Stores a new chat with its title; returns its number.
pub fn chat(db: &UserDb, title: &str) -> Result<i64, DbError> {
    let connection = db.connection();

    connection.execute(&format!("INSERT INTO chats (title, created) VALUES (?1, {NOW})"), [title])?;

    Ok(connection.last_insert_rowid())
}

/// Adds a message of the evaluator to a chat.
pub fn message(db: &UserDb, chat_id: i64, text: &str) -> Result<(), DbError> {
    db.connection().execute(
        &format!("INSERT INTO messages (chat_id, speaker, text, created) VALUES (?1, ?2, ?3, {NOW})"),
        params![chat_id, speaker_name(Speaker::Evaluator), text],
    )?;

    Ok(())
}

/// A reply of the model, as it is stored.
pub struct Reply<'a> {
    pub text: &'a str,
    pub model: Option<&'a str>,
    pub ending: &'a Ending,
    pub note: Option<&'a str>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Adds a reply of the model to a chat.
pub fn reply(db: &UserDb, chat_id: i64, reply: &Reply<'_>) -> Result<(), DbError> {
    db.connection().execute(
        &format!(
            "INSERT INTO messages (chat_id, speaker, text, created, model, ending, note, input_tokens, output_tokens)
             VALUES (?1, ?2, ?3, {NOW}, ?4, ?5, ?6, ?7, ?8)"
        ),
        params![
            chat_id,
            speaker_name(Speaker::Model),
            reply.text,
            reply.model,
            reply.ending.name(),
            reply.note,
            reply.input_tokens as i64,
            reply.output_tokens as i64,
        ],
    )?;

    Ok(())
}
