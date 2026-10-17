//! Removing settings and secrets of an evaluator.

use crate::db::error::DbError;

use super::database::UserDb;

impl UserDb {
    /// Removes a setting; a setting that is not stored is not an
    /// error.
    pub fn remove_setting(&self, name: &str) -> Result<(), DbError> {
        self.connection().execute("DELETE FROM settings WHERE name = ?1", [name])?;

        Ok(())
    }

    /// Removes a secret; a secret that is not stored is not an
    /// error.
    ///
    /// SQLite marks the space of a removed row as free but does not
    /// overwrite it at once; `secure_delete` makes it overwrite the
    /// value with zeros, so that the old secret is not left in the
    /// (encrypted) file.
    pub fn remove_secret(&self, name: &str) -> Result<(), DbError> {
        self.connection().execute_batch("PRAGMA secure_delete = ON")?;
        self.connection().execute("DELETE FROM secrets WHERE name = ?1", [name])?;

        Ok(())
    }
}
