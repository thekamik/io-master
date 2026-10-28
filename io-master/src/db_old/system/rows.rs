//! The rows of the system database and the Rust types they become.

use crate::db_old::encryption::keys::WrappedKey;
use crate::db_old::encryption::recovery::RecoveryWrappedKey;

/// An account as the system database knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub id: i64,
    pub name: String,
    /// The key of the database of the account, wrapped with the password.
    pub key: WrappedKey,
    /// The same key, wrapped with the recovery code.
    pub recovery: RecoveryWrappedKey,
}

/// Reads one row of the `users` table.
pub(super) fn user_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRecord> {
    Ok(UserRecord {
        id: row.get("id")?,
        name: row.get("name")?,
        key: WrappedKey {
            salt: row.get("key_salt")?,
            memory_kib: row.get("key_memory_kib")?,
            iterations: row.get("key_iterations")?,
            nonce: row.get("key_nonce")?,
            ciphertext: row.get("key_ciphertext")?,
            tag: row.get("key_tag")?,
        },
        recovery: RecoveryWrappedKey {
            salt: row.get("recovery_salt")?,
            nonce: row.get("recovery_nonce")?,
            ciphertext: row.get("recovery_ciphertext")?,
            tag: row.get("recovery_tag")?,
        },
    })
}
