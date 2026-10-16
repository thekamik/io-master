//! The AI connections of an evaluator, kept in their encrypted
//! database (`db/user_db.rs`):
//!
//! ```text
//! ai_connections                 the fields of every connection
//! secrets:  ai.connection.<id>.api_key   the API key of one
//! settings: ai.active_connection         the one the chat uses
//! ```
//!
//! The database is opened only with the key that the password of
//! the evaluator unwraps (`db/keys.rs`), so the API keys are
//! protected at least by that password.
//!
//! **A connection is stored only when it keeps the rules**
//! ([`Connection::check`]). Whether the service answers is not
//! needed: a local model may be switched off while it is set up; the
//! check is a separate step (`ai/client.rs`, [`check`]).
//!
//! **An API key is bound to the address it was entered for**: the
//! scheme, host and port of the URL ([`Connection::origin`]). When a
//! connection is saved with another address, its key is removed, so
//! that a changed URL never sends the key to another service.
//!
//! [`check`]: super::client::check

use std::fmt;

use rusqlite::types::ToSql;
use rusqlite::{OptionalExtension, Row, params};

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

use super::api_key::ApiKey;
use super::connection::{AnswerMode, Connection, ConnectionError};

/// Name of the setting with the active connection.
const ACTIVE: &str = "ai.active_connection";

/// The columns of a connection, in the order of [`from_row`].
const COLUMNS: &str = "id, name, url, model, ca_certificate, headers, body, user_role, model_role, \
system_role, system_prompt, answer, text_path, stop_path, cut_reasons, refused_reasons, error_path, \
model_path, input_tokens_path, output_tokens_path, done_marker, message_template";

/// Name of the secret with the key of a connection.
fn key_name(id: i64) -> String {
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

/// What saving did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub id: i64,
    /// The API key was removed because the address changed.
    pub key_removed: bool,
}

/// A connection from a row with [`COLUMNS`].
fn from_row(row: &Row<'_>) -> rusqlite::Result<(i64, Connection)> {
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

/// Whether a key is stored for a connection.
fn has_key(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.secret(&key_name(id))?.is_some())
}

/// Every connection, by name.
pub fn list(db: &UserDb) -> Result<Vec<Stored>, DbError> {
    let mut statement = db
        .connection()
        .prepare(&format!("SELECT {COLUMNS} FROM ai_connections ORDER BY name COLLATE NOCASE, id"))?;

    let rows = statement.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(|(id, connection)| {
            Ok(Stored {
                id,
                connection,
                has_key: has_key(db, id)?,
            })
        })
        .collect()
}

/// One connection, or `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Stored>, DbError> {
    let row = db
        .connection()
        .query_row(&format!("SELECT {COLUMNS} FROM ai_connections WHERE id = ?1"), [id], from_row)
        .optional()?;

    match row {
        Some((id, connection)) => Ok(Some(Stored {
            id,
            connection,
            has_key: has_key(db, id)?,
        })),
        None => Ok(None),
    }
}

/// Saves a connection: a new one when `id` is `None`, else changes
/// that one. The first connection becomes the active one.
pub fn save(db: &UserDb, id: Option<i64>, connection: &Connection) -> Result<Saved, SaveError> {
    connection.check().map_err(SaveError::Invalid)?;

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
    let taken: Option<i64> = db
        .connection()
        .query_row(
            "SELECT id FROM ai_connections WHERE name = ?1 COLLATE NOCASE",
            [name],
            |row| row.get(0),
        )
        .optional()
        .map_err(DbError::from)?;

    if taken.is_some_and(|other| Some(other) != id) {
        return Err(SaveError::NameTaken);
    }

    // The values in the order of the columns of the INSERT and the
    // UPDATE below (?1 to ?21).
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

    let sql = db.connection();

    let (id, key_removed) = match id {
        None => {
            sql.execute(
                "INSERT INTO ai_connections (name, url, model, ca_certificate, headers, body, user_role, \
                 model_role, system_role, system_prompt, answer, text_path, stop_path, cut_reasons, \
                 refused_reasons, error_path, model_path, input_tokens_path, output_tokens_path, done_marker, \
                 message_template) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                &values[..],
            )
            .map_err(DbError::from)?;

            (sql.last_insert_rowid(), false)
        }
        Some(id) => {
            let origin: Option<Option<String>> = sql
                .query_row("SELECT key_origin FROM ai_connections WHERE id = ?1", [id], |row| row.get(0))
                .optional()
                .map_err(DbError::from)?;

            let Some(origin) = origin else {
                return Err(SaveError::NoSuchConnection);
            };

            sql.execute(
                "UPDATE ai_connections SET name = ?1, url = ?2, model = ?3, ca_certificate = ?4, headers = ?5, \
                 body = ?6, user_role = ?7, model_role = ?8, system_role = ?9, system_prompt = ?10, answer = ?11, \
                 text_path = ?12, stop_path = ?13, cut_reasons = ?14, refused_reasons = ?15, error_path = ?16, \
                 model_path = ?17, input_tokens_path = ?18, output_tokens_path = ?19, done_marker = ?20, \
                 message_template = ?21 WHERE id = ?22",
                &[&values[..], &[&id as &dyn ToSql]].concat()[..],
            )
            .map_err(DbError::from)?;

            // The key stays only with the address it was entered for.
            let moved = has_key(db, id)? && origin != connection.origin();

            if moved {
                remove_api_key(db, id)?;
            }

            (id, moved)
        }
    };

    if active(db)?.is_none() {
        db.set_setting(ACTIVE, &id.to_string())?;
    }

    Ok(Saved { id, key_removed })
}

/// Deletes a connection with its key; returns whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    remove_api_key(db, id)?;

    let deleted = db.connection().execute("DELETE FROM ai_connections WHERE id = ?1", [id])? > 0;

    if deleted && db.setting(ACTIVE)?.as_deref() == Some(id.to_string().as_str()) {
        db.connection().execute("DELETE FROM settings WHERE name = ?1", [ACTIVE])?;
    }

    Ok(deleted)
}

/// The active connection, or `None` when none is chosen (or the
/// chosen one no longer exists).
pub fn active(db: &UserDb) -> Result<Option<i64>, DbError> {
    let Some(id) = db.setting(ACTIVE)?.and_then(|text| text.parse::<i64>().ok()) else {
        return Ok(None);
    };

    Ok(get(db, id)?.map(|stored| stored.id))
}

/// Makes a connection the active one; returns whether it exists.
pub fn set_active(db: &UserDb, id: i64) -> Result<bool, DbError> {
    if get(db, id)?.is_none() {
        return Ok(false);
    }

    db.set_setting(ACTIVE, &id.to_string())?;

    Ok(true)
}

/// The API key of a connection, or `None` when none is stored.
///
/// A stored text that is no longer a valid key (impossible unless
/// the database was changed by hand) counts as no key.
pub fn api_key(db: &UserDb, id: i64) -> Result<Option<ApiKey>, DbError> {
    Ok(db
        .secret(&key_name(id))?
        .and_then(|text| ApiKey::new(text.as_str().to_owned()).ok()))
}

/// Stores the API key of a connection, bound to its address; returns
/// whether the connection exists.
pub fn store_api_key(db: &UserDb, id: i64, key: &ApiKey) -> Result<bool, DbError> {
    let Some(stored) = get(db, id)? else {
        return Ok(false);
    };

    db.set_secret(&key_name(id), key.expose())?;
    db.connection().execute(
        "UPDATE ai_connections SET key_origin = ?1 WHERE id = ?2",
        params![stored.connection.origin(), id],
    )?;

    Ok(true)
}

/// Removes the API key of a connection (overwritten in the file).
pub fn remove_api_key(db: &UserDb, id: i64) -> Result<(), DbError> {
    db.remove_secret(&key_name(id))?;
    db.connection()
        .execute("UPDATE ai_connections SET key_origin = NULL WHERE id = ?1", [id])?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "connections_tests.rs"]
mod tests;
