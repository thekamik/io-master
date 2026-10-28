//! The user of the installation: one Linux account holds one
//! installation with one user. Kept in the table `users` of the system
//! database, which is not encrypted, so it holds nothing secret. The
//! keys that open the encrypted database of the user (its copies
//! wrapped with the password and with the recovery code) are kept with
//! it, but belong to `db/encryption/` and are no part of this model.

/// The user of the installation (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code, reason = "a new model (2026-10-28), not used by the program yet, only by db/ and its tests")]
pub struct User {
    /// The id of the user in the database; it names the file of the
    /// encrypted database of the user.
    pub id: i64,

    /// The name the user logs in with, unique whatever its letter case.
    pub name: String,

    /// When the user was made: seconds since 1970 (UTC).
    pub created: i64,
}
