//! The database of the user: the file `users/<id>.v2.db` of the data
//! folder, where it is, made and opened, its tables made when they are
//! missing (`connection.rs`, `models/schema.rs`: `USER_TABLES`). It holds
//! everything but the login: the plans, the agents, the API keys, the
//! journal. The layout changes freely until version 1 (CLAUDE.md): a
//! database of an older layout is removed and made anew.
//!
//! **Protection: encrypted by SQLCipher** (`encryption/sqlcipher.rs`) with
//! the key of the user, unwrapped by the password at the login.
//!
//! ```text
//! access.rs: keys_and_database ──create(data_dir, id, key)─► sqlcipher::create ─┐
//! access.rs: log_in            ──open(data_dir, id, key)───► sqlcipher::open ───┴─► from ─► UserDatabase
//! ```

use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::models::schema::USER_TABLES;
use crate::db::storage::connection::prepare;
use crate::db::storage::encryption::database_key::DatabaseKey;
use crate::db::storage::encryption::sqlcipher;

/// The folder of the databases of the users, in the data folder.
const DIRECTORY: &str = "users";

/// The open database of the user: the connection, unlocked with the key.
/// Held by the session while the user is logged in; every function of
/// `db/models/` on the data of the user takes it.
#[derive(Debug)]
pub struct UserDatabase {
    connection: Connection,
}

impl UserDatabase {
    /// Makes the database of the user from a connection just opened.
    ///
    /// Input: `connection`, the file of the database of the user, unlocked
    /// with its key.
    ///
    /// Output: the database, every table of `USER_TABLES` made; an error of
    /// SQLite when a table cannot be made.
    ///
    /// Called by: [`create`], [`open`].
    /// Calls: `connection.rs`: `prepare`.
    fn from(connection: Connection) -> Result<UserDatabase, DbError> {
        prepare(&connection, &USER_TABLES)?;

        Ok(UserDatabase { connection })
    }

    /// The connection, for SQL. **Only `db/` reaches it**
    /// (`pub(in crate::db)`): SQL written anywhere else does not compile
    /// (CLAUDE.md, "The database").
    ///
    /// Called by: every function of `models/ai/`, `models/evaluation/` and
    /// `models/user/program_data.rs` (the settings) that runs SQL.
    pub(in crate::db) fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the database of the user `user_id`: `<data>/users/7.v2.db`.
///
/// Called by: [`create`], [`open`]; `panels/logic/program_start/access.rs`:
/// `create_account` (the file removed when the first start fails half way).
pub fn path(data_dir: &Path, user_id: i64) -> PathBuf {
    data_dir.join(DIRECTORY).join(format!("{user_id}.v2.db"))
}

/// Makes the empty database of the user at the first start.
///
/// Input:
/// - `data_dir`: the data folder of the program (`~/.local/share/io-master`).
/// - `user_id`: the id of the user in the system database (1).
/// - `key`: the new random key of the database (`panels/logic/program_start/generate_key.rs`).
///
/// Output: the database, open, with its tables. An error when the file
/// exists already (never overwritten: `DbError::Exists`) or when the folder
/// or the file cannot be made.
///
/// Called by: `panels/logic/program_start/access.rs`: `keys_and_database`;
/// the tests: `test_support/database.rs`.
/// Calls: [`path`], `sqlcipher::create`, [`UserDatabase::from`].
pub fn create(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDatabase, DbError> {
    let directory = data_dir.join(DIRECTORY);

    // The folder of the databases readable only by the user of the
    // computer (0700); an existing one is kept.
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .map_err(|source| DbError::Io { path: directory.clone(), source })?;

    UserDatabase::from(sqlcipher::create(&path(data_dir, user_id), key)?)
}

/// Opens the database of the user at the login.
///
/// Input:
/// - `data_dir`: the data folder of the program.
/// - `user_id`: the id of the user in the system database (1).
/// - `key`: the key of the database, unwrapped by the password
///   (`panels/logic/program_start/keys.rs`: `unwrap`).
///
/// Output: the database, open, with its tables. An error when the file is
/// missing (`DbError::NotFound`) or the key does not open it
/// (`DbError::WrongKey`).
///
/// Called by: `panels/logic/program_start/access.rs`: `log_in`.
/// Calls: [`path`], `sqlcipher::open`, [`UserDatabase::from`].
pub fn open(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDatabase, DbError> {
    UserDatabase::from(sqlcipher::open(&path(data_dir, user_id), key)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the database of the user: made, opened again.

    use super::*;

    use crate::security::random::fill_random;
    use crate::test_support::TempDir;

    #[test]
    fn a_database_is_made_once_and_opened_again() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

        create(directory.path(), 1, &key).unwrap();

        assert!(directory.path().join("users").join("1.v2.db").exists());
        assert!(matches!(create(directory.path(), 1, &key), Err(DbError::Exists(_))));
        open(directory.path(), 1, &key).unwrap();
    }

    #[test]
    fn another_key_does_not_open_it() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let other = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

        create(directory.path(), 1, &key).unwrap();

        assert!(matches!(open(directory.path(), 1, &other), Err(DbError::WrongKey(_))));
    }
}
