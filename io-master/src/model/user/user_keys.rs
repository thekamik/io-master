//! The keys of the database of a user, as they are kept in the system
//! database (`db/models/user/user_keys.rs`): two copies of the one random key of
//! the encrypted database of the user, one wrapped with the password, one
//! with the recovery code (`panels/logic/program_start/keys.rs`, `panels/logic/program_start/recovery.rs`). None
//! of it is secret: a copy is useless without its password or its code.
//! The user decided (2026-10-28) that the keys are a model of their own,
//! apart from `User`, and that the two copies keep the shape they had.

/// The two copies of the key of the database of a user (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserKeys {
    /// The user they open the database of (`User`).
    pub user: i64,

    /// The copy wrapped with the password.
    pub password: WrappedKey,

    /// The copy wrapped with the recovery code.
    pub recovery: RecoveryWrappedKey,
}

/// The key of a database, encrypted with an unlocking key.
/// Everything needed to open it again, except the password; none of
/// it is secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedKey {
    /// The random salt of Argon2id.
    pub salt: Vec<u8>,
    /// The parameters of Argon2id used, so that they can be raised
    /// later without locking out existing accounts.
    pub memory_kib: u32,
    pub iterations: u32,
    /// The nonce of AES-GCM: random, used once.
    pub nonce: Vec<u8>,
    /// The encrypted key and its authentication tag.
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
}

/// The key of a database, wrapped with a recovery code. Everything
/// needed to open it again, except the code; none of it is secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryWrappedKey {
    /// The random salt of HKDF.
    pub salt: Vec<u8>,
    /// The nonce of AES-GCM: random, used once.
    pub nonce: Vec<u8>,
    /// The encrypted key and its authentication tag.
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
}
