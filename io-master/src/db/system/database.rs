//! The system database: `system.db` in the data directory.
//!
//! It is the record of the installation (`docs/architecture.md`,
//! sections 3.2 and 5.3): which installation this is, its role,
//! and its accounts. Its existence decides whether this is the
//! first start.
//!
//! **Protection: NOT encrypted.** The program opens it at every
//! start, before anyone has given a password, so no key could open
//! it (`encryption/sqlcipher.rs`, `create_plain`). It therefore holds
//! nothing secret: names, a role, and the keys of the databases of
//! the evaluators only in encrypted form (`encryption/keys.rs`), which
//! are useless without the passwords. The file is readable only by
//! the user.
//!
//! TODO (step 8 of docs/todo.md): with the optional TPM factor it
//! can be encrypted with a key from the secret of the device.
//!
//! ```text
//! database.rs   the file: create, open, remove; the lock
//! schema.rs     the tables
//! rows.rs       a row of `users` ↔ UserRecord
//! create.rs     the record of the installation, a new account
//! read.rs       the installation, the accounts
//! update.rs     the key of an account, after a change of password
//! delete.rs     an account
//! ```
//!
//! Every account has its own database (`db/user/`) under
//! `users/<id>.db`; this database holds only what is needed to open
//! it.
//!
//! The connection is kept behind a lock, so that the commands of
//! the interface, which may run at the same time, use it one after
//! another.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::db::encryption::sqlcipher;
use crate::db::error::DbError;
use crate::installation::installation::Installation;

use super::{create, schema};

/// Name of the file of the system database.
pub const FILE_NAME: &str = "system.db";

/// The open system database.
#[derive(Debug)]
pub struct SystemDb {
    connection: Mutex<Connection>,
}

/// The path of the system database in a data directory.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

impl SystemDb {
    /// Creates the system database at the first start, with the
    /// record of the installation. Fails when one exists.
    pub fn create(data_dir: &Path, installation: &Installation) -> Result<SystemDb, DbError> {
        let connection = sqlcipher::create_plain(&path(data_dir))?;

        connection.execute_batch(schema::SCHEMA)?;
        create::installation(&connection, installation)?;

        Ok(SystemDb { connection: Mutex::new(connection) })
    }

    /// Opens the system database of a data directory.
    pub fn open(data_dir: &Path) -> Result<SystemDb, DbError> {
        let connection = sqlcipher::open_plain(&path(data_dir))?;

        Ok(SystemDb { connection: Mutex::new(connection) })
    }

    /// Removes the system database; used when the program is
    /// uninstalled. A missing file is not an error.
    pub fn remove(data_dir: &Path) -> Result<(), DbError> {
        let path = path(data_dir);

        match fs::remove_file(&path) {
            Err(source) if source.kind() != io::ErrorKind::NotFound => Err(DbError::Io { path, source }),
            _ => Ok(()),
        }
    }

    /// The connection, for one piece of work; only the files of this
    /// folder reach it. A panic in another request while it held the
    /// lock does not leave the database half-changed (SQLite rolls
    /// back an unfinished transaction), so the lock is used as it is.
    pub(super) fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items. They
// cover the whole folder (create, read, update, delete).
#[cfg(test)]
#[path = "database_tests.rs"]
mod tests;
