//! The database of one evaluator: `users/<id>.db` in the data
//! directory.
//!
//! **Protection: encrypted by SQLCipher** (`encryption/sqlcipher.rs`)
//! with a random key, stored only encrypted with the password of the
//! evaluator and the secret of the application (`encryption/keys.rs`),
//! so nobody else, not even another evaluator of the same
//! workstation, can open it. Every area of the folders below keeps its
//! tables in this one file:
//!
//! ```text
//! db/user/            settings, secrets            (this folder)
//! db/ai/              AI: connections, memory of the chat, roles,
//!                     instructions, journal, presets
//! db/standards/       documents of standards
//! db/test_projects/   test projects, schemes, tiles, links
//! db/evaluations/     evaluation projects, results, journal
//! ```
//!
//! ```text
//! database.rs   the file: create, open, remove; the tables of all areas
//! schema.rs     settings, secrets
//! read.rs       a setting, a secret
//! update.rs     a setting, a secret stored (made or replaced)
//! delete.rs     a setting, a secret; what earlier versions kept
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

use rusqlite::Connection;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::encryption::sqlcipher;
use crate::db::error::DbError;
use crate::db::{ai, evaluations, standards, test_projects};

use super::{delete, schema};

/// Name of the folder of the databases of the evaluators.
pub const DIRECTORY: &str = "users";

/// The open database of one evaluator.
#[derive(Debug)]
pub struct UserDb {
    connection: Connection,
}

impl UserDb {
    /// Prepares an open connection: the tables that are missing, in
    /// every area.
    fn from(connection: Connection) -> Result<UserDb, DbError> {
        connection.execute_batch(schema::SCHEMA)?;
        connection.execute_batch(ai::connection::schema::SCHEMA)?;
        connection.execute_batch(ai::memory::schema::SCHEMA)?;
        connection.execute_batch(ai::roles::schema::SCHEMA)?;
        connection.execute_batch(ai::instructions::schema::SCHEMA)?;
        connection.execute_batch(ai::journal::schema::SCHEMA)?;
        connection.execute_batch(standards::schema::SCHEMA)?;
        connection.execute_batch(test_projects::schema::SCHEMA)?;
        connection.execute_batch(evaluations::schema::SCHEMA)?;
        delete::remove_obsolete(&connection)?;

        let has_template: bool = connection.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('ai_connections') WHERE name = 'message_template'",
            [],
            |row| row.get(0),
        )?;

        if !has_template {
            connection.execute_batch(ai::connection::schema::MESSAGE_TEMPLATE_COLUMN)?;
        }

        Ok(UserDb { connection })
    }

    /// The connection, for the files of `db/` that keep their tables
    /// here. **Only `db/` reaches it** (`pub(in crate::db)`): SQL
    /// written anywhere else does not compile (CLAUDE.md, "Database
    /// code").
    pub(in crate::db) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The connection for a test that damages a database on purpose,
    /// to see how the program copes. Compiled only in the tests.
    #[cfg(test)]
    pub fn connection_for_tests(&self) -> &Connection {
        &self.connection
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
        .map_err(|source| DbError::Io { path: directory.clone(), source })?;

    UserDb::from(sqlcipher::create(&path(data_dir, user_id), key)?)
}

/// Opens the database of an account with its key.
pub fn open(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDb, DbError> {
    UserDb::from(sqlcipher::open(&path(data_dir, user_id), key)?)
}

/// Removes the databases of all accounts; used when the program is
/// uninstalled. A missing folder is not an error.
pub fn remove_all(data_dir: &Path) -> Result<(), DbError> {
    let directory = data_dir.join(DIRECTORY);

    match fs::remove_dir_all(&directory) {
        Err(source) if source.kind() != io::ErrorKind::NotFound => Err(DbError::Io { path: directory, source }),
        _ => Ok(()),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items. They
// cover the whole folder (read, update, delete).
#[cfg(test)]
#[path = "database_tests.rs"]
mod tests;
