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

#[cfg(test)]
mod tests {
    //! Tests of storing the settings and secrets of an evaluator.

    use super::*;

    use crate::db::user::read;
    use crate::test_support::standards::database;

    #[test]
    fn a_setting_is_stored_and_replaced() {
        let test = database();

        set_setting(&test.db, "scan.columns", "path,type").unwrap();
        set_setting(&test.db, "scan.columns", "path").unwrap();

        assert_eq!(read::setting(&test.db, "scan.columns").unwrap().as_deref(), Some("path"));
    }

    #[test]
    fn a_secret_is_stored_and_replaced() {
        let test = database();

        set_secret(&test.db, "api", "first").unwrap();
        set_secret(&test.db, "api", "second").unwrap();

        assert_eq!(read::secret(&test.db, "api").unwrap().as_deref().map(String::as_str), Some("second"));
    }
}
