//! Making a new AI connection. **A connection is stored only when it
//! keeps the rules** ([`Connection::check`]); whether the service
//! answers is not needed: a local model may be switched off while it
//! is set up, and the check is a separate step (`ai/api/checks.rs`).

use crate::ai::api::connection::Connection;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::read;
use super::rows::{ACTIVE, SaveError, Saved, with_connection_values};

/// Stores a new connection. Its name must be free; the first
/// connection becomes the active one. One transaction: the row and
/// the choice of the active connection are stored together or not at
/// all.
pub fn connection(db: &UserDb, connection: &Connection) -> Result<Saved, SaveError> {
    connection.check().map_err(SaveError::Invalid)?;

    if read::named(db, connection.name.trim())?.is_some() {
        return Err(SaveError::NameTaken);
    }

    let transaction = db.connection().unchecked_transaction().map_err(DbError::from)?;

    with_connection_values(connection, |values| {
        transaction.execute(
            "INSERT INTO ai_connections (name, url, model, ca_certificate, headers, body, user_role, \
             model_role, system_role, system_prompt, answer, text_path, stop_path, cut_reasons, \
             refused_reasons, error_path, model_path, input_tokens_path, output_tokens_path, done_marker, \
             message_template) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            values,
        )
    })
    .map_err(|error| SaveError::Db(error.into()))?;

    let id = transaction.last_insert_rowid();

    if read::active(db)?.is_none() {
        db.set_setting(ACTIVE, &id.to_string())?;
    }

    transaction.commit().map_err(DbError::from)?;

    Ok(Saved { id, key_removed: false })
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, update, delete).
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
