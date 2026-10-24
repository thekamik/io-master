//! The tables of the system database (`database.rs`). They are made
//! once, when the database is created. **Not encrypted**: nothing in
//! them is secret (the keys of the accounts are stored only wrapped,
//! `encryption/keys.rs`).
//!
//! ```text
//! installation   one row: identifier, time of the first start
//! users          one row: the account, its name, time it was created,
//!                and the encrypted key of its database
//! ```
//!
//! A database made before step 48 of docs/todo.md has two columns more,
//! `installation.role` and `users.key_device` (the role master and the
//! TPM factor, both removed). The program never reads them, and writes
//! these tables only when the database is made, so such a database
//! keeps working unchanged.

/// The tables, created with the database.
pub const SCHEMA: &str = "
    CREATE TABLE installation (
        id          BLOB    NOT NULL,
        created     INTEGER NOT NULL
    );

    CREATE TABLE users (
        id              INTEGER PRIMARY KEY,
        -- Names are unique whatever their letter case: 'Anna'
        -- and 'anna' would be too easy to mix up.
        name            TEXT    NOT NULL UNIQUE COLLATE NOCASE,
        created         INTEGER NOT NULL,
        key_salt        BLOB    NOT NULL,
        key_memory_kib  INTEGER NOT NULL,
        key_iterations  INTEGER NOT NULL,
        key_nonce       BLOB    NOT NULL,
        key_ciphertext  BLOB    NOT NULL,
        key_tag         BLOB    NOT NULL
    );
";
