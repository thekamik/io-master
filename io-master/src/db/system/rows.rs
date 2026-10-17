//! The rows of the system database and the Rust types they become.

use crate::db::encryption::keys::WrappedKey;

/// An account as the system database knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub id: i64,
    pub name: String,
    /// The encrypted key of the database of the account.
    pub key: WrappedKey,
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
            with_device_secret: row.get("key_device")?,
            nonce: row.get("key_nonce")?,
            ciphertext: row.get("key_ciphertext")?,
            tag: row.get("key_tag")?,
        },
    })
}
