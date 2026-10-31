//! The database of the user: the file `users/<id>.v2.db` of the data
//! folder, where it is, made and opened; its tables made when they are
//! missing (`models/schema.rs`, `USER_TABLES`). The layout changes freely
//! until version 1 (CLAUDE.md): a database of an older layout is removed
//! and made anew.
//!
//! **Protection: encrypted by SQLCipher** (`encryption/sqlcipher.rs`) with
//! the key of the user.

use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::models::schema::USER_TABLES;
use crate::db::storage::encryption::database_key::DatabaseKey;
use crate::db::storage::encryption::sqlcipher;

/// How long an operation waits for another that holds the database.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The folder of the databases of the users, in the data folder.
const DIRECTORY: &str = "users";

/// The open database of the user.
#[derive(Debug)]
pub struct UserDatabase {
    connection: Connection,
}

impl UserDatabase {
    /// Prepares an open connection: the tables of `USER_TABLES` that are
    /// missing.
    ///
    /// Called by: [`create`], [`open`].
    fn from(connection: Connection) -> Result<UserDatabase, DbError> {
        // Two operations may use the database at the same time; the second
        // waits for the first instead of failing at once.
        connection.busy_timeout(BUSY_TIMEOUT)?;

        for table in USER_TABLES {
            connection.execute_batch(table)?;
        }

        Ok(UserDatabase { connection })
    }

    /// The connection, for the files of `db/`. **Only `db/` reaches it**:
    /// SQL written anywhere else does not compile (CLAUDE.md, "The
    /// database").
    pub(in crate::db) fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the database of the user `user_id`.
///
/// Called by: [`create`], [`open`].
pub fn path(data_dir: &Path, user_id: i64) -> PathBuf {
    data_dir.join(DIRECTORY).join(format!("{user_id}.v2.db"))
}

/// Makes the empty database of the user `user_id`, its folder private
/// to the user of the computer; an existing file is never overwritten.
///
/// Calls: `sqlcipher::create`, [`UserDatabase::from`].
pub fn create(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDatabase, DbError> {
    let directory = data_dir.join(DIRECTORY);

    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .map_err(|source| DbError::Io { path: directory.clone(), source })?;

    UserDatabase::from(sqlcipher::create(&path(data_dir, user_id), key)?)
}

/// Opens the database of the user `user_id` with its key.
///
/// Calls: `sqlcipher::open`, [`UserDatabase::from`].
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
    fn a_database_is_made_beside_the_earlier_one_and_opened_again() {
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
