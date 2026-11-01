//! The system database: the file `system.v2.db` of the data folder, where
//! it is, made and opened, its tables made when they are missing
//! (`connection.rs`; the list of the tables given by the caller,
//! `objects/schema.rs`: `SYSTEM_TABLES`). It holds what the login needs:
//! the installation, its one user and the two wrapped copies of the key of
//! the database of the user (`objects/user/`).
//!
//! **Protection: NOT encrypted.** The program opens it at every start,
//! before anyone has given a password, so no key could open it; it holds
//! nothing secret. The file is readable only by the user of the computer
//! (`encryption/sqlcipher.rs`, `create_plain`).
//!
//! ```text
//! objects/user/user_data/user_data_rules.rs:
//!   create_account          ──create(data_dir, SYSTEM_TABLES)─► sqlcipher::create_plain ─┐
//!   installed_user, log_in  ──open(data_dir, SYSTEM_TABLES)───► sqlcipher::open_plain ───┴─► from ─► SystemDatabase
//! ```

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::infrastructure::storage::connection::prepare;
use crate::infrastructure::storage::encryption::sqlcipher;
use crate::infrastructure::storage::error::DbError;

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
    /// Input:
    /// - `connection`: the plain file of the system database.
    /// - `tables`: the `SCHEMA` of every table of the system database
    ///   (`objects/schema.rs`: `SYSTEM_TABLES`).
    ///
    /// Output: the database, every table of `tables` made; an error of
    /// SQLite when a table cannot be made.
    ///
    /// Called by: [`create`], [`open`].
    /// Calls: `connection.rs`: `prepare`.
    fn from(connection: Connection, tables: &[&str]) -> Result<SystemDatabase, DbError> {
        prepare(&connection, tables)?;

        Ok(SystemDatabase { connection })
    }

    /// The connection, for SQL. **Only `db/models/` reaches it**: checked by
    /// `architecture_tests.rs` (`the_sql_is_only_in_the_store`), since the
    /// models in the databases are no longer below this folder.
    ///
    /// Called by: every function of `objects/user/user_data/user_data_store.rs`,
    /// `objects/user/user_keys/user_keys_store.rs` and `objects/user/program_data/program_data_store.rs`
    /// (the installation) that runs SQL.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the system database: `<data>/system.v2.db`.
///
/// Called by: [`create`], [`open`]; `objects/user/user_data/user_data_rules.rs`:
/// `create_account` (the file removed when the first start fails half way).
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Makes the empty system database at the first start.
///
/// Input:
/// - `data_dir`: the data folder of the program (`~/.local/share/io-master`).
/// - `tables`: the `SCHEMA` of every table of the system database
///   (`objects/schema.rs`: `SYSTEM_TABLES`).
///
/// Output: the database, open, with its tables. An error when the file
/// exists already (never overwritten: `DbError::Exists`, the program is
/// installed) or cannot be made.
///
/// Called by: `objects/user/user_data/user_data_rules.rs`: `create_account`.
/// Calls: [`path`], `sqlcipher::create_plain`, [`SystemDatabase::from`].
pub fn create(data_dir: &Path, tables: &[&str]) -> Result<SystemDatabase, DbError> {
    SystemDatabase::from(sqlcipher::create_plain(&path(data_dir))?, tables)
}

/// Opens the system database at the start and at the login.
///
/// Input:
/// - `data_dir`: the data folder of the program.
/// - `tables`: the `SCHEMA` of every table of the system database
///   (`objects/schema.rs`: `SYSTEM_TABLES`).
///
/// Output: the database, open, with its tables. An error when the file is
/// missing (`DbError::NotFound`: the program is not installed yet) or is
/// not a plain database of SQLite (`DbError::WrongKey`).
///
/// Called by: `objects/user/user_data/user_data_rules.rs`: `installed_user`,
/// `log_in`.
/// Calls: [`path`], `sqlcipher::open_plain`, [`SystemDatabase::from`].
pub fn open(data_dir: &Path, tables: &[&str]) -> Result<SystemDatabase, DbError> {
    SystemDatabase::from(sqlcipher::open_plain(&path(data_dir))?, tables)
}

#[cfg(test)]
mod tests {
    //! Tests of the system database: made once, opened again.

    use super::*;

    use crate::objects::schema::SYSTEM_TABLES;
    use crate::test_support::TempDir;

    #[test]
    fn the_system_database_is_made_once_and_opened_again() {
        let directory = TempDir::new();

        create(directory.path(), &SYSTEM_TABLES).unwrap();

        assert!(directory.path().join("system.v2.db").exists());
        assert!(matches!(create(directory.path(), &SYSTEM_TABLES), Err(DbError::Exists(_))));
        open(directory.path(), &SYSTEM_TABLES).unwrap();
    }
}
