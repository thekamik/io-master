//! Creating rows of the system database: the record of the
//! installation (once, with the database) and the accounts.

use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::db::encryption::keys::WrappedKey;
use crate::db::error::DbError;
use crate::installation::installation::Installation;

use super::database::SystemDb;

/// Seconds since 1970, the time stamps of the database.
fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| i64::try_from(time.as_secs()).unwrap_or(i64::MAX))
}

/// Writes the record of the installation into a new database
/// (`SystemDb::create`).
pub(super) fn installation(connection: &Connection, installation: &Installation) -> Result<(), DbError> {
    connection.execute(
        "INSERT INTO installation (id, role, created) VALUES (?1, ?2, ?3)",
        params![installation.id.as_slice(), installation.role.name(), now()],
    )?;

    Ok(())
}

impl SystemDb {
    /// Adds an account and returns its id. Fails when the name is
    /// taken.
    pub fn add_user(&self, name: &str, key: &WrappedKey) -> Result<i64, DbError> {
        let connection = self.connection();

        connection.execute(
            "INSERT INTO users (name, created, key_salt, key_memory_kib, key_iterations,
                                key_device, key_nonce, key_ciphertext, key_tag)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                name,
                now(),
                key.salt,
                key.memory_kib,
                key.iterations,
                key.with_device_secret,
                key.nonce,
                key.ciphertext,
                key.tag
            ],
        )?;

        Ok(connection.last_insert_rowid())
    }
}
