//! The system database: `system.db` in the data directory.
//!
//! It is the record of the installation (`docs/architecture.md`,
//! sections 3.2 and 5.3): which installation this is and its
//! account. Its existence decides whether this is the
//! first start.
//!
//! **Protection: NOT encrypted.** The program opens it at every
//! start, before anyone has given a password, so no key could open
//! it (`encryption/sqlcipher.rs`, `create_plain`). It therefore holds
//! nothing secret: a name and the key of the database of
//! the evaluator only in encrypted form (`encryption/keys.rs`), which
//! are useless without the passwords. The file is readable only by
//! the user.
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

use crate::db_old::encryption::sqlcipher;
use crate::db_old::error::DbError;
use crate::db_old::user::database::BUSY_TIMEOUT;
use crate::model_old::installation::Installation;

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
        connection.busy_timeout(BUSY_TIMEOUT)?;

        connection.execute_batch(schema::SCHEMA)?;
        create::installation(&connection, installation)?;

        Ok(SystemDb { connection: Mutex::new(connection) })
    }

    /// Opens the system database of a data directory.
    pub fn open(data_dir: &Path) -> Result<SystemDb, DbError> {
        let connection = sqlcipher::open_plain(&path(data_dir))?;
        connection.busy_timeout(BUSY_TIMEOUT)?;

        Ok(SystemDb { connection: Mutex::new(connection) })
    }

    /// Removes the system database; a missing file is not an error.
    /// Called by `installation/lifecycle.rs` when the first start
    /// fails half-way, so that the next start is a first start again.
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

#[cfg(test)]
mod tests {
    //! Tests of the system database: made, opened again, private, removed.

    use super::*;

    use crate::db_old::system::{create, read};
    use crate::test_support::TempDir;
    use crate::test_support::db::{installation, recovery_key, system_db, wrapped_key};

    #[test]
    fn the_installation_is_kept() {
        let directory = TempDir::new();
        system_db(&directory);

        let reopened = SystemDb::open(directory.path()).unwrap();
        assert_eq!(read::installation(&reopened).unwrap(), installation());
    }

    #[test]
    fn it_holds_no_secret_and_is_private() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new();
        let db = system_db(&directory);
        create::add_user(&db, "anna", &wrapped_key(), &recovery_key()).unwrap();

        // A plain SQLite file, readable only by the user.
        let bytes = fs::read(path(directory.path())).unwrap();
        assert!(bytes.starts_with(b"SQLite format 3"));
        assert_eq!(fs::metadata(path(directory.path())).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn removing_the_database_leaves_nothing() {
        let directory = TempDir::new();
        system_db(&directory);

        SystemDb::remove(directory.path()).unwrap();

        assert!(!path(directory.path()).exists());
        SystemDb::remove(directory.path()).unwrap();
    }
}
