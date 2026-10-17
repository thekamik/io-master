//! The rows of the AI connections and the Rust types they become
//! (`ai/api/connection.rs`), the names under which the key and the active
//! connection are kept, and the types the functions of this folder
//! give back.

use std::fmt;

use rusqlite::Row;
use rusqlite::types::ToSql;

use crate::ai::api::connection::{AnswerMode, Connection, ConnectionError};
use crate::db::error::DbError;

/// Name of the setting with the active connection (`db/user/`).
pub(super) const ACTIVE: &str = "ai.active_connection";

/// Name of the secret with the key of a connection (`db/user/`).
pub(super) fn key_name(id: i64) -> String {
    format!("ai.connection.{id}.api_key")
}

/// A stored connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub id: i64,
    pub connection: Connection,
    /// Whether an API key is stored for it (never the key itself).
    pub has_key: bool,
}

/// What saving did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub id: i64,
    /// The API key was removed because the address changed.
    pub key_removed: bool,
}

/// Why a connection was not saved.
#[derive(Debug)]
pub enum SaveError {
    /// It breaks a rule.
    Invalid(ConnectionError),
    /// Another connection has this name.
    NameTaken,
    /// The connection to change does not exist.
    NoSuchConnection,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for SaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Invalid(error) => write!(formatter, "{error}"),
            SaveError::NameTaken => formatter.write_str("another connection has this name"),
            SaveError::NoSuchConnection => formatter.write_str("the connection does not exist"),
            SaveError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<DbError> for SaveError {
    fn from(error: DbError) -> SaveError {
        SaveError::Db(error)
    }
}

/// The columns of a connection, in the order of [`connection_from_row`].
pub(super) const CONNECTION_COLUMNS: &str = "id, name, url, model, ca_certificate, headers, body, user_role, \
model_role, system_role, system_prompt, answer, text_path, stop_path, cut_reasons, refused_reasons, error_path, \
model_path, input_tokens_path, output_tokens_path, done_marker, message_template";

/// A connection from a row with [`CONNECTION_COLUMNS`].
pub(super) fn connection_from_row(row: &Row<'_>) -> rusqlite::Result<(i64, Connection)> {
    let answer: String = row.get(11)?;

    Ok((
        row.get(0)?,
        Connection {
            name: row.get(1)?,
            url: row.get(2)?,
            model: row.get(3)?,
            ca_certificate: row.get(4)?,
            headers: row.get(5)?,
            body: row.get(6)?,
            user_role: row.get(7)?,
            model_role: row.get(8)?,
            system_role: row.get(9)?,
            system_prompt: row.get(10)?,
            // Only valid names are written; a changed database falls
            // back to the stream.
            answer: AnswerMode::from_name(&answer).unwrap_or(AnswerMode::Stream),
            text_path: row.get(12)?,
            stop_path: row.get(13)?,
            cut_reasons: row.get(14)?,
            refused_reasons: row.get(15)?,
            error_path: row.get(16)?,
            model_path: row.get(17)?,
            input_tokens_path: row.get(18)?,
            output_tokens_path: row.get(19)?,
            done_marker: row.get(20)?,
            message_template: row.get(21)?,
        },
    ))
}

/// Calls `write` with the values of a connection as they are stored
/// (the single-line fields trimmed), in the order of the columns of
/// the INSERT and the UPDATE (?1 to ?21).
pub(super) fn with_connection_values<T>(connection: &Connection, write: impl FnOnce(&[&dyn ToSql]) -> T) -> T {
    let name = connection.name.trim();
    let url = connection.url.trim();
    let model = connection.model.trim();
    let ca_certificate = connection.ca_certificate.trim();
    let user_role = connection.user_role.trim();
    let model_role = connection.model_role.trim();
    let system_role = connection.system_role.trim();
    let answer = connection.answer.name();
    let text_path = connection.text_path.trim();
    let stop_path = connection.stop_path.trim();
    let cut_reasons = connection.cut_reasons.trim();
    let refused_reasons = connection.refused_reasons.trim();
    let error_path = connection.error_path.trim();
    let model_path = connection.model_path.trim();
    let input_tokens_path = connection.input_tokens_path.trim();
    let output_tokens_path = connection.output_tokens_path.trim();
    let done_marker = connection.done_marker.trim();

    let values: [&dyn ToSql; 21] = [
        &name,
        &url,
        &model,
        &ca_certificate,
        &connection.headers,
        &connection.body,
        &user_role,
        &model_role,
        &system_role,
        &connection.system_prompt,
        &answer,
        &text_path,
        &stop_path,
        &cut_reasons,
        &refused_reasons,
        &error_path,
        &model_path,
        &input_tokens_path,
        &output_tokens_path,
        &done_marker,
        &connection.message_template,
    ];

    write(&values)
}
