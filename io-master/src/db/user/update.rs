//! Storing the settings and secrets of an evaluator. A value is made
//! when it is missing and replaced when it exists (one statement,
//! `ON CONFLICT ... DO UPDATE`), so storing is an update here.

use rusqlite::named_params;

use crate::db::error::DbError;

use super::database::UserDb;

/// Stores a setting, replacing an earlier value.
pub fn set_setting(db: &UserDb, name: &str, value: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO settings (name, value) VALUES (:name, :value)
         ON CONFLICT (name) DO UPDATE SET value = excluded.value",
        named_params! { ":name": name, ":value": value },
    )?;

    Ok(())
}

/// Stores a secret, replacing an earlier value.
pub fn set_secret(db: &UserDb, name: &str, value: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO secrets (name, value) VALUES (:name, :value)
         ON CONFLICT (name) DO UPDATE SET value = excluded.value",
        named_params! { ":name": name, ":value": value },
    )?;

    Ok(())
}
