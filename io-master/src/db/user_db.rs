//! The database of one evaluator: `users/<id>.db` in the data
//! directory.
//!
//! Everything an evaluator works with is kept here: later the API
//! keys, the chats, the settings, the results. Its key is random
//! and stored only encrypted with the password of the evaluator
//! and the secret of the device (`keys.rs`), so nobody else, not
//! even another evaluator of the same workstation, can open it.
//!
//! It holds no tables yet; each step of `docs/todo.md` that keeps
//! data of an evaluator adds its own.

use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::database_key::DatabaseKey;
use super::db::{self, DbError};

/// Name of the folder of the databases of the evaluators.
pub const DIRECTORY: &str = "users";

/// The path of the database of an account.
pub fn path(data_dir: &Path, user_id: i64) -> PathBuf {
    data_dir.join(DIRECTORY).join(format!("{user_id}.db"))
}

/// Creates the empty database of a new account.
pub fn create(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<Connection, DbError> {
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

    db::create(&path(data_dir, user_id), key)
}

/// Opens the database of an account with its key.
pub fn open(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<Connection, DbError> {
    db::open(&path(data_dir, user_id), key)
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
