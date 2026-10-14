//! The database of one evaluator: `users/<id>.db` in the data
//! directory.
//!
//! Everything an evaluator works with is kept here: later the API
//! keys, the chats, the settings, the results. Its key is random
//! and stored only encrypted with the password of the evaluator
//! and the secret of the device (`keys.rs`), so nobody else, not
//! even another evaluator of the same workstation, can open it.
//!
//! ```text
//! settings   name -> value: the settings of the evaluator, for
//!            example of the scanner (devices/scan_settings.rs)
//! ```
//!
//! Each step of `docs/todo.md` that keeps data of an evaluator adds
//! its own tables. Tables are created with `IF NOT EXISTS` when the
//! database is opened, so a database made by an earlier version
//! gets the tables it lacks.

use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use super::database_key::DatabaseKey;
use super::db::{self, DbError};

/// Name of the folder of the databases of the evaluators.
pub const DIRECTORY: &str = "users";

/// The tables of the database.
const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS settings (
        name    TEXT PRIMARY KEY,
        value   TEXT NOT NULL
    );
";

/// The open database of one evaluator.
#[derive(Debug)]
pub struct UserDb {
    connection: Connection,
}

impl UserDb {
    /// Prepares an open connection: the tables that are missing.
    fn from(connection: Connection) -> Result<UserDb, DbError> {
        connection.execute_batch(SCHEMA)?;

        Ok(UserDb { connection })
    }

    /// The value of a setting, or `None` when it was never stored.
    pub fn setting(&self, name: &str) -> Result<Option<String>, DbError> {
        Ok(self
            .connection
            .query_row("SELECT value FROM settings WHERE name = ?1", [name], |row| row.get(0))
            .optional()?)
    }

    /// Stores a setting, replacing an earlier value.
    pub fn set_setting(&self, name: &str, value: &str) -> Result<(), DbError> {
        self.connection.execute(
            "INSERT INTO settings (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = excluded.value",
            params![name, value],
        )?;

        Ok(())
    }
}

/// The path of the database of an account.
pub fn path(data_dir: &Path, user_id: i64) -> PathBuf {
    data_dir.join(DIRECTORY).join(format!("{user_id}.db"))
}

/// Creates the empty database of a new account.
pub fn create(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDb, DbError> {
    let directory = data_dir.join(DIRECTORY);

    // The folder is private to the user of the computer.
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .map_err(|source| DbError::Io {
            path: directory.clone(),
            source,
        })?;

    UserDb::from(db::create(&path(data_dir, user_id), key)?)
}

/// Opens the database of an account with its key.
pub fn open(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDb, DbError> {
    UserDb::from(db::open(&path(data_dir, user_id), key)?)
}

/// Removes the databases of all accounts; used when the program is
/// uninstalled. A missing folder is not an error.
pub fn remove_all(data_dir: &Path) -> Result<(), DbError> {
    let directory = data_dir.join(DIRECTORY);

    match fs::remove_dir_all(&directory) {
        Err(source) if source.kind() != io::ErrorKind::NotFound => Err(DbError::Io {
            path: directory,
            source,
        }),
        _ => Ok(()),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "user_db_tests.rs"]
mod tests;
