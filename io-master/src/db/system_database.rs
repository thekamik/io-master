//! The system database, made anew for the models of `model/` (2026-10-28):
//! the file `system.v2.db` of the data folder, beside the earlier
//! `system.db` of `db_old/`, which the login keeps using until it is
//! rewritten. It holds the installation and its one user
//! (`db/user/`).
//!
//! **Protection: NOT encrypted.** The program opens it at every start,
//! before anyone has given a password, so no key could open it; it holds
//! nothing secret. The file is readable only by the user of the computer
//! (`encryption/sqlcipher.rs`, `create_plain`). The copies of the key of
//! the database of the user will be kept beside the user when the login
//! is rewritten on `db/`; they belong to `db/encryption/`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::db::encryption::sqlcipher;
use crate::db::error::DbError;
use crate::db::user::{program_data, user_data, user_keys};

/// How long an operation waits for another that holds the database.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The name of the file of the system database.
const FILE_NAME: &str = "system.v2.db";

/// The open system database.
#[derive(Debug)]
pub struct SystemDatabase {
    connection: Connection,
}

impl SystemDatabase {
    /// Prepares an open connection: the tables that are missing.
    ///
    /// Called by: [`create`], [`open`].
    fn from(connection: Connection) -> Result<SystemDatabase, DbError> {
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.execute_batch(program_data::SYSTEM_SCHEMA)?;
        connection.execute_batch(user_data::SCHEMA)?;
        connection.execute_batch(user_keys::SCHEMA)?;

        Ok(SystemDatabase { connection })
    }

    /// The connection, for the files of `db/`. **Only `db/` reaches it**.
    pub(in crate::db) fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the system database in the data folder `data_dir`.
///
/// Called by: [`create`], [`open`].
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Makes the empty system database at the first start; an existing file
/// is never overwritten.
///
/// Calls: `sqlcipher::create_plain`, [`SystemDatabase::from`].
pub fn create(data_dir: &Path) -> Result<SystemDatabase, DbError> {
    SystemDatabase::from(sqlcipher::create_plain(&path(data_dir))?)
}

/// Opens the system database of the data folder `data_dir`.
///
/// Calls: `sqlcipher::open_plain`, [`SystemDatabase::from`].
pub fn open(data_dir: &Path) -> Result<SystemDatabase, DbError> {
    SystemDatabase::from(sqlcipher::open_plain(&path(data_dir))?)
}

#[cfg(test)]
mod tests {
    //! Tests of the system database: made once, opened again.

    use super::*;

    use crate::test_support::TempDir;

    #[test]
    fn the_system_database_is_made_once_and_opened_again() {
        let directory = TempDir::new();

        create(directory.path()).unwrap();

        assert!(directory.path().join("system.v2.db").exists());
        assert!(matches!(create(directory.path()), Err(DbError::Exists(_))));
        open(directory.path()).unwrap();
    }
}
