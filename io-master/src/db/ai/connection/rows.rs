//! The rows of the AI connections and the Rust types they become
//! (`ai/api/connection.rs`), the names under which the key and the active
//! connection are kept, and the types the functions of this folder
//! give back.

use std::fmt;

use rusqlite::Row;
use rusqlite::types::ToSql;

use crate::db::error::{DbError, named};
use crate::model_old::ai::{AnswerMode, Connection};

/// Name of the setting with the active connection (`db/user/`).
pub(super) const ACTIVE: &str = "ai.active_connection";

/// Name of the secret with the key of a connection (`db/user/`).
pub(super) fn key_name(id: i64) -> String {
    format!("ai.connection.{id}.api_key")
}

/// Why a connection was not stored: the rules of the stored data
/// (`ai/api/connections.rs` checks the fields before).
#[derive(Debug)]
pub enum SaveError {
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

/// A connection from a row of `ai_connections`, with its number.
pub(super) fn connection_from_row(row: &Row<'_>) -> rusqlite::Result<(i64, Connection)> {
    let answer: String = row.get("answer")?;

    Ok((
        row.get("id")?,
        Connection {
            name: row.get("name")?,
            url: row.get("url")?,
            model: row.get("model")?,
            ca_certificate: row.get("ca_certificate")?,
            headers: row.get("headers")?,
            body: row.get("body")?,
            user_role: row.get("user_role")?,
            model_role: row.get("model_role")?,
            system_role: row.get("system_role")?,
            system_prompt: row.get("system_prompt")?,
            answer: named("ai_connections", "answer", &answer, AnswerMode::from_name)?,
            text_path: row.get("text_path")?,
            stop_path: row.get("stop_path")?,
            cut_reasons: row.get("cut_reasons")?,
            refused_reasons: row.get("refused_reasons")?,
            error_path: row.get("error_path")?,
            model_path: row.get("model_path")?,
            input_tokens_path: row.get("input_tokens_path")?,
            output_tokens_path: row.get("output_tokens_path")?,
            done_marker: row.get("done_marker")?,
            message_template: row.get("message_template")?,
        },
    ))
}

/// The columns a connection is stored in (all but `id` and
/// `key_origin`); the parameter of each is its name with a colon.
pub(super) const COLUMNS: [&str; 21] = [
    "name",
    "url",
    "model",
    "ca_certificate",
    "headers",
    "body",
    "user_role",
    "model_role",
    "system_role",
    "system_prompt",
    "answer",
    "text_path",
    "stop_path",
    "cut_reasons",
    "refused_reasons",
    "error_path",
    "model_path",
    "input_tokens_path",
    "output_tokens_path",
    "done_marker",
    "message_template",
];

/// Calls `write` with the values of a connection, as they come (in
/// their normal form, `model/ai.rs`), by the names of [`COLUMNS`].
pub(super) fn with_connection_values<T>(connection: &Connection, write: impl FnOnce(&[(&str, &dyn ToSql)]) -> T) -> T {
    let answer = connection.answer.name();
    let values: [(&str, &dyn ToSql); 21] = [
        (":name", &connection.name),
        (":url", &connection.url),
        (":model", &connection.model),
        (":ca_certificate", &connection.ca_certificate),
        (":headers", &connection.headers),
        (":body", &connection.body),
        (":user_role", &connection.user_role),
        (":model_role", &connection.model_role),
        (":system_role", &connection.system_role),
        (":system_prompt", &connection.system_prompt),
        (":answer", &answer),
        (":text_path", &connection.text_path),
        (":stop_path", &connection.stop_path),
        (":cut_reasons", &connection.cut_reasons),
        (":refused_reasons", &connection.refused_reasons),
        (":error_path", &connection.error_path),
        (":model_path", &connection.model_path),
        (":input_tokens_path", &connection.input_tokens_path),
        (":output_tokens_path", &connection.output_tokens_path),
        (":done_marker", &connection.done_marker),
        (":message_template", &connection.message_template),
    ];

    write(&values)
}
