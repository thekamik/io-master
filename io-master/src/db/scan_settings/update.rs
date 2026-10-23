//! Storing the settings of the scanner.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::{COLUMNS, PORT_TYPES};

/// The longest text stored (64 kinds of port of at most 32 characters,
/// with their commas, fit with room to spare).
pub const MAX_TEXT: usize = 4096;

/// Stores both texts, replacing the earlier ones, in one transaction:
/// either both are stored or neither.
pub fn texts(db: &UserDb, port_types: &str, columns: &str) -> Result<(), DbError> {
    ensure!(is_stored_text(port_types), DbError::Inconsistent("the kinds of port are no text to store".to_owned()));
    ensure!(is_stored_text(columns), DbError::Inconsistent("the columns are no text to store".to_owned()));

    let transaction = db.connection().unchecked_transaction()?;
    let statement = "INSERT INTO settings (name, value) VALUES (:name, :value)
                     ON CONFLICT (name) DO UPDATE SET value = excluded.value";

    transaction.execute(statement, named_params! { ":name": PORT_TYPES, ":value": port_types })?;
    transaction.execute(statement, named_params! { ":name": COLUMNS, ":value": columns })?;
    transaction.commit()?;

    Ok(())
}

/// Whether `text` can be stored: not empty, and not longer than
/// [`MAX_TEXT`].
fn is_stored_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_TEXT
}

// Unit tests of the whole area, in their own file.
#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
