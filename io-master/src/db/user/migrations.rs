//! The changes of the layout of the database of an evaluator, by its
//! version (`PRAGMA user_version`, step 40 of docs/todo.md).
//!
//! A new database starts at version 1 (`encryption/sqlcipher.rs`,
//! `SCHEMA_VERSION`). When a database is opened, its missing tables
//! are made (`CREATE TABLE IF NOT EXISTS`, in `database.rs`) and then
//! every step below its version runs once, in one transaction, and
//! the version is raised. A database already at [`VERSION`] runs
//! nothing.
//!
//! **The history starts again at version 1** (step 48 of docs/todo.md,
//! 2026-10-24: the user began with a fresh installation). The steps
//! 2 to 4 of the development (the column of the template of a message,
//! the tables of the model before OSCAL, the settings of the scanner of
//! serial ports) served only the databases of that time and were
//! removed; what they made is in the tables themselves now. A database
//! of a version this program does not know (one of the development, at
//! version 2 to 4, or of a newer program) is refused, never opened half
//! understood.
//!
//! ```text
//! (no steps yet: the first change of the layout is 1 → 2)
//! ```

use rusqlite::Connection;

use crate::db::error::{Corrupt, DbError};

/// The version of the layout this program writes.
pub const VERSION: i64 = 1;

/// Brings the database to [`VERSION`]: today only the check that it is
/// a version this program knows.
///
/// Called by: `UserDb::from` (`database.rs`), whenever a database is
/// opened or made, after its tables. Calls nothing but SQLite.
pub(super) fn run(connection: &Connection) -> Result<(), DbError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    ensure!(
        version == VERSION,
        DbError::Corrupt(Corrupt { table: "user_version", column: "user_version", value: version.to_string() })
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `migrations.rs`.

    use super::*;

    use crate::db::encryption::database_key::DatabaseKey;
    use crate::db::user::database::{self as user_db, UserDb};
    use crate::security::random::fill_random;
    use crate::test_support::TempDir;

    /// A new database, and the key that opens it again.
    fn database(directory: &TempDir) -> (UserDb, DatabaseKey) {
        let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
        let db = user_db::create(directory.path(), 1, &key).unwrap();
        let again = DatabaseKey::fill_with(|bytes| {
            bytes.copy_from_slice(key.expose());
            Ok::<(), ()>(())
        })
        .unwrap();

        (db, again)
    }

    fn version(db: &UserDb) -> i64 {
        db.connection_for_tests().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn a_new_database_is_at_the_latest_version_and_opens_again() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        assert_eq!(version(&db), VERSION);
        drop(db);

        assert!(user_db::open(directory.path(), 1, &key).is_ok());
    }

    #[test]
    fn a_database_of_the_development_is_refused() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        db.connection_for_tests().execute_batch("PRAGMA user_version = 4").unwrap();
        drop(db);

        assert!(matches!(user_db::open(directory.path(), 1, &key), Err(DbError::Corrupt(_))));
    }
}
