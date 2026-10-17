//! Changing rows of the system database: the key of an account.

use rusqlite::params;

use crate::db::encryption::keys::WrappedKey;
use crate::db::error::DbError;

use super::database::SystemDb;

impl SystemDb {
    /// Stores a newly encrypted key of an account, after a change
    /// of its password.
    pub fn replace_user_key(&self, id: i64, key: &WrappedKey) -> Result<(), DbError> {
        let changed = self.connection().execute(
            "UPDATE users SET key_salt = ?2, key_memory_kib = ?3, key_iterations = ?4,
                              key_device = ?5, key_nonce = ?6, key_ciphertext = ?7, key_tag = ?8
             WHERE id = ?1",
            params![
                id,
                key.salt,
                key.memory_kib,
                key.iterations,
                key.with_device_secret,
                key.nonce,
                key.ciphertext,
                key.tag
            ],
        )?;

        if changed == 1 {
            Ok(())
        } else {
            Err(DbError::Sql(format!("no account {id}")))
        }
    }
}
