//! Making a new AI connection. **A connection is stored only when it
//! keeps the rules** ([`Connection::check`]); whether the service
//! answers is not needed: a local model may be switched off while it
//! is set up, and the check is a separate step (`ai/api/checks.rs`).

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Connection;

use super::read;
use super::rows::{ACTIVE, COLUMNS, SaveError, Saved, with_connection_values};

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
            &format!(
                "INSERT INTO ai_connections ({}) VALUES ({})",
                COLUMNS.join(", "),
                COLUMNS.map(|column| format!(":{column}")).join(", ")
            ),
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
