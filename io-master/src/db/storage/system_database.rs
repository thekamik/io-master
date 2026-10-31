//! The system database: the file `system.v2.db` of the data folder, where
//! it is, made and opened, its tables made when they are missing
//! (`connection.rs`, `models/schema.rs`: `SYSTEM_TABLES`). It holds what
//! the login needs: the installation, its one user and the two wrapped
//! copies of the key of the database of the user (`models/user/`).
//!
//! **Protection: NOT encrypted.** The program opens it at every start,
//! before anyone has given a password, so no key could open it; it holds
//! nothing secret. The file is readable only by the user of the computer
//! (`encryption/sqlcipher.rs`, `create_plain`).
//!
//! ```text
//! access.rs: create_account          ──create(data_dir)─► sqlcipher::create_plain ─┐
//! access.rs: installed_user, log_in  ──open(data_dir)───► sqlcipher::open_plain ───┴─► from ─► SystemDatabase
//! ```

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::db::models::schema::SYSTEM_TABLES;
use crate::db::storage::connection::prepare;
use crate::db::storage::encryption::sqlcipher;

/// The name of the file of the system database, in the data folder.
const FILE_NAME: &str = "system.v2.db";

/// The open system database: the connection to its plain file. Opened for
/// the first start and the login, and closed when they end.
#[derive(Debug)]
pub struct SystemDatabase {
    connection: Connection,
}

impl SystemDatabase {
    /// Makes the system database from a connection just opened.
    ///
    /// Input: `connection`, the plain file of the system database.
    ///
    /// Output: the database, every table of `SYSTEM_TABLES` made; an error
    /// of SQLite when a table cannot be made.
    ///
    /// Called by: [`create`], [`open`].
    /// Calls: `connection.rs`: `prepare`.
    fn from(connection: Connection) -> Result<SystemDatabase, DbError> {
        prepare(&connection, &SYSTEM_TABLES)?;

        Ok(SystemDatabase { connection })
    }

    /// The connection, for SQL. **Only `db/` reaches it**
    /// (`pub(in crate::db)`).
    ///
    /// Called by: every function of `models/user/user_data.rs`,
    /// `models/user/user_keys.rs` and `models/user/program_data.rs` (the
    /// installation) that runs SQL.
    pub(in crate::db) fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the system database: `<data>/system.v2.db`.
///
/// Called by: [`create`], [`open`]; `panels/logic/program_start/access.rs`:
/// `create_account` (the file removed when the first start fails half way).
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Makes the empty system database at the first start.
///
/// Input: `data_dir`, the data folder of the program
/// (`~/.local/share/io-master`).
///
/// Output: the database, open, with its tables. An error when the file
/// exists already (never overwritten: `DbError::Exists`, the program is
/// installed) or cannot be made.
///
/// Called by: `panels/logic/program_start/access.rs`: `create_account`.
/// Calls: [`path`], `sqlcipher::create_plain`, [`SystemDatabase::from`].
pub fn create(data_dir: &Path) -> Result<SystemDatabase, DbError> {
    SystemDatabase::from(sqlcipher::create_plain(&path(data_dir))?)
}

/// Opens the system database at the start and at the login.
///
/// Input: `data_dir`, the data folder of the program.
///
/// Output: the database, open, with its tables. An error when the file is
/// missing (`DbError::NotFound`: the program is not installed yet) or is
/// not a plain database of SQLite (`DbError::WrongKey`).
///
/// Called by: `panels/logic/program_start/access.rs`: `installed_user`,
/// `log_in`.
/// Calls: [`path`], `sqlcipher::open_plain`, [`SystemDatabase::from`].
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
