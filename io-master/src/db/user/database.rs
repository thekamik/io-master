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
//! db/ai/              AI: connections, chats, roles,
//!                     instructions, journal, presets
//! db/standards/       documents of standards
//! db/oscal/           documents of NIST OSCAL
//! db/evaluation_support/  quotes and proposals linked to them
//! db/conversion/      the conversions of documents of standards into catalogs
//! ```
//!
//! ```text
//! database.rs   the file: create, open, remove; the tables of all areas
//! schema.rs     settings, secrets
//! read.rs       a setting, a secret
//! update.rs     a setting, a secret stored (made or replaced)
//! delete.rs     a setting, a secret
//! migrations.rs the changes of the layout, by version
//! ```
//!
//! Each step of `docs/todo.md` that keeps data of an evaluator adds
//! its own tables. Tables are created with `IF NOT EXISTS` when the
//! database is opened, so a database made by an earlier version
//! gets the tables it lacks; other changes of the layout are the steps
//! of `migrations.rs`.

use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::encryption::sqlcipher;
use crate::db::error::DbError;
use crate::db::{ai, conversion, evaluation_support, oscal, standards};

use super::{migrations, schema};

/// How long a command waits for another that holds the database.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Name of the folder of the databases of the evaluators.
pub const DIRECTORY: &str = "users";

/// The open database of one evaluator.
#[derive(Debug)]
pub struct UserDb {
    connection: Connection,
}

impl UserDb {
    /// Prepares an open connection: the tables that are missing, in
    /// every area, and the changes of the layout (`migrations.rs`).
    fn from(connection: Connection) -> Result<UserDb, DbError> {
        // Two commands may use the database at the same time; the second
        // waits for the first instead of failing at once.
        connection.busy_timeout(BUSY_TIMEOUT)?;

        connection.execute_batch(schema::SCHEMA)?;
        connection.execute_batch(ai::connection::schema::SCHEMA)?;
        connection.execute_batch(ai::chats::schema::SCHEMA)?;
        connection.execute_batch(ai::roles::schema::SCHEMA)?;
        connection.execute_batch(ai::instructions::schema::SCHEMA)?;
        connection.execute_batch(ai::journal::schema::SCHEMA)?;
        connection.execute_batch(standards::schema::SCHEMA)?;
        connection.execute_batch(oscal::schema::SCHEMA)?;
        connection.execute_batch(evaluation_support::schema::SCHEMA)?;
        connection.execute_batch(conversion::schema::SCHEMA)?;
        migrations::run(&connection)?;

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

/// Runs `work` in one transaction of database `db`: every function of
/// `db/` it calls writes into the transaction, which is committed when
/// `work` succeeds and rolled back when it fails. For an operation of
/// the business made of several writes that belong together (an
/// accepted proposal: the object, its quotes, the proposal removed).
pub fn in_transaction<T, E: From<DbError>>(db: &UserDb, work: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let transaction = db.connection.unchecked_transaction().map_err(DbError::from)?;
    let result = work()?;

    transaction.commit().map_err(DbError::from)?;

    Ok(result)
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

#[cfg(test)]
mod tests {
    //! Tests of the database of an evaluator: made, opened, kept, removed.

    use super::*;

    use std::os::unix::fs::PermissionsExt;

    use crate::db::user::{read, update};
    use crate::security::random::fill_random;
    use crate::test_support::TempDir;

    #[test]
    fn a_database_of_an_account_is_made_and_opened() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

        create(directory.path(), 3, &key).unwrap();

        assert!(path(directory.path(), 3).ends_with("users/3.db"));
        assert!(open(directory.path(), 3, &key).is_ok());

        let mode = fs::metadata(directory.path().join(DIRECTORY)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn what_is_stored_is_kept_in_the_file_and_read_after_opening_again() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let db = create(directory.path(), 1, &key).unwrap();
        update::set_setting(&db, "scan.columns", "path").unwrap();
        drop(db);

        // Kept in the file, and readable only with the key.
        let db = open(directory.path(), 1, &key).unwrap();
        assert_eq!(read::setting(&db, "scan.columns").unwrap().as_deref(), Some("path"));
    }

    #[test]
    fn all_databases_are_removed() {
        let directory = TempDir::new();
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

        create(directory.path(), 1, &key).unwrap();
        create(directory.path(), 2, &key).unwrap();
        remove_all(directory.path()).unwrap();

        assert!(!directory.path().join(DIRECTORY).exists());
        remove_all(directory.path()).unwrap();
    }
}
