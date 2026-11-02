//! The database of the user: the file `users/<id>.v2.db` of the data
//! folder, where it is, made and opened, its tables made when they are
//! missing (`connection.rs`; the list of the tables given by the caller,
//! `objects/schema.rs`: `USER_TABLES`). It holds
//! everything but the login: the plans, the agents, the API keys, the
//! journal. The layout changes freely until version 1 (CLAUDE.md): a
//! database of an older layout is removed and made anew.
//!
//! **Protection: encrypted by `SQLCipher`** (`encryption/sqlcipher.rs`) with
//! the key of the user, unwrapped by the password at the login.
//!
//! ```text
//! objects/user/user_data/user_data_rules.rs:
//!   keys_and_database ──create(data_dir, id, key, USER_TABLES)─► sqlcipher::create ─┐
//!   log_in            ──open(data_dir, id, key, USER_TABLES)───► sqlcipher::open ───┴─► from ─► UserDatabase
//! ```

use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::infrastructure::storage::connection::prepare;
use crate::infrastructure::storage::encryption::database_key::DatabaseKey;
use crate::infrastructure::storage::encryption::sqlcipher;
use crate::infrastructure::storage::error::DbError;

/// The folder of the databases of the users, in the data folder.
const DIRECTORY: &str = "users";

/// The open database of the user: the connection, unlocked with the key.
/// Held by the session while the user is logged in; every function of
/// the stores (`<object>_store.rs`) on the data of the user takes it.
#[derive(Debug)]
pub struct UserDatabase {
    connection: Connection,
}

impl UserDatabase {
    /// Makes the database of the user from a connection just opened.
    ///
    /// Input:
    /// - `connection`: the file of the database of the user, unlocked with
    ///   its key.
    /// - `tables`: the `SCHEMA` of every table of the database of the user
    ///   (`objects/schema.rs`: `USER_TABLES`).
    ///
    /// Output: the database, every table of `tables` made; an error of
    /// SQLite when a table cannot be made.
    ///
    /// Called by: [`create`], [`open`].
    /// Calls: `infrastructure/storage/connection.rs`: `prepare`.
    fn from(connection: Connection, tables: &[&str]) -> Result<UserDatabase, DbError> {
        prepare(&connection, tables)?;

        Ok(UserDatabase { connection })
    }

    /// The connection, for SQL. **Only the stores reach it**
    /// (CLAUDE.md, "The database"): checked by `architecture_tests.rs`
    /// (`the_sql_is_only_in_the_store`), since the models in the databases
    /// are no longer below this folder.
    ///
    /// Called by: every function of the stores that runs SQL on the data of
    /// the user: `objects/ai/*/<object>_store.rs`, `objects/evaluation/*/<object>_store.rs`,
    /// `objects/user/program_data/program_data_store.rs` (the settings).
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the database of the user `user_id`: `<data>/users/7.v2.db`.
///
/// Called by: [`create`], [`open`]; `objects/user/user_data/user_data_rules.rs`:
/// `create_account` (the file removed when the first start fails half way).
pub fn path(data_dir: &Path, user_id: i64) -> PathBuf {
    data_dir.join(DIRECTORY).join(format!("{user_id}.v2.db"))
}

/// Makes the empty database of the user at the first start.
///
/// Input:
/// - `data_dir`: the data folder of the program (`~/.local/share/io-master`).
/// - `user_id`: the id of the user in the system database (1).
/// - `key`: the new random key of the database (`infrastructure/storage/encryption/generate_key.rs`).
/// - `tables`: the `SCHEMA` of every table of the database of the user
///   (`objects/schema.rs`: `USER_TABLES`).
///
/// Output: the database, open, with its tables. An error when the file
/// exists already (never overwritten: `DbError::Exists`) or when the folder
/// or the file cannot be made.
///
/// Called by: `objects/user/user_data/user_data_rules.rs`: `keys_and_database`;
/// the tests: `test_support/database.rs`.
/// Calls: [`path`]; `infrastructure/storage/encryption/sqlcipher.rs`: `create`; [`UserDatabase::from`].
pub fn create(data_dir: &Path, user_id: i64, key: &DatabaseKey, tables: &[&str]) -> Result<UserDatabase, DbError> {
    let directory = data_dir.join(DIRECTORY);

    // The folder of the databases readable only by the user of the
    // computer (0700); an existing one is kept.
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .map_err(|source| DbError::Io { path: directory.clone(), source })?;

    UserDatabase::from(sqlcipher::create(&path(data_dir, user_id), key)?, tables)
}

/// Opens the database of the user at the login.
///
/// Input:
/// - `data_dir`: the data folder of the program.
/// - `user_id`: the id of the user in the system database (1).
/// - `key`: the key of the database, unwrapped by the password
///   (`objects/user/user_keys/user_keys_rules/password.rs`: `unwrap`).
/// - `tables`: the `SCHEMA` of every table of the database of the user
///   (`objects/schema.rs`: `USER_TABLES`).
///
/// Output: the database, open, with its tables. An error when the file is
/// missing (`DbError::NotFound`) or the key does not open it
/// (`DbError::WrongKey`).
///
/// Called by: `objects/user/user_data/user_data_rules.rs`: `log_in`.
/// Calls: [`path`]; `infrastructure/storage/encryption/sqlcipher.rs`: `open`; [`UserDatabase::from`].
pub fn open(data_dir: &Path, user_id: i64, key: &DatabaseKey, tables: &[&str]) -> Result<UserDatabase, DbError> {
    UserDatabase::from(sqlcipher::open(&path(data_dir, user_id), key)?, tables)
}

#[cfg(test)]
mod tests {
    //! Tests of the database of the user: made, opened again.

    use super::*;

    use crate::infrastructure::security::random::fill_random;
    use crate::objects::schema::USER_TABLES;
    use crate::test_support::TempDir;

    #[test]
    fn a_database_is_made_once_and_opened_again() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

        create(directory.path(), 1, &key, &USER_TABLES).unwrap();

        assert!(directory.path().join("users").join("1.v2.db").exists());
        assert!(matches!(create(directory.path(), 1, &key, &USER_TABLES), Err(DbError::Exists(_))));
        open(directory.path(), 1, &key, &USER_TABLES).unwrap();
    }

    #[test]
    fn another_key_does_not_open_it() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let other = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

        create(directory.path(), 1, &key, &USER_TABLES).unwrap();

        assert!(matches!(open(directory.path(), 1, &other, &USER_TABLES), Err(DbError::WrongKey(_))));
    }
}
