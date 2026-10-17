//! Reading the settings and secrets of an evaluator.

use rusqlite::OptionalExtension;
use zeroize::Zeroizing;

use crate::db::error::DbError;

use super::database::UserDb;

impl UserDb {
    /// The value of a setting, or `None` when it was never stored.
    pub fn setting(&self, name: &str) -> Result<Option<String>, DbError> {
        Ok(self
            .connection()
            .query_row("SELECT value FROM settings WHERE name = ?1", [name], |row| row.get(0))
            .optional()?)
    }

    /// The value of a secret, or `None` when none is stored. The
    /// copy given out is wiped when it is dropped.
    pub fn secret(&self, name: &str) -> Result<Option<Zeroizing<String>>, DbError> {
        Ok(self
            .connection()
            .query_row("SELECT value FROM secrets WHERE name = ?1", [name], |row| row.get::<_, String>(0))
            .optional()?
            .map(Zeroizing::new))
    }
}
