//! Storing the settings and secrets of an evaluator. A value is made
//! when it is missing and replaced when it exists (one statement,
//! `ON CONFLICT ... DO UPDATE`), so storing is an update here.

use rusqlite::params;

use crate::db::error::DbError;

use super::database::UserDb;

impl UserDb {
    /// Stores a setting, replacing an earlier value.
    pub fn set_setting(&self, name: &str, value: &str) -> Result<(), DbError> {
        self.connection().execute(
            "INSERT INTO settings (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = excluded.value",
            params![name, value],
        )?;

        Ok(())
    }

    /// Stores a secret, replacing an earlier value.
    pub fn set_secret(&self, name: &str, value: &str) -> Result<(), DbError> {
        self.connection().execute(
            "INSERT INTO secrets (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = excluded.value",
            params![name, value],
        )?;

        Ok(())
    }
}
