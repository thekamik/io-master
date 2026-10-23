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
//! ```text
//! 1 → 2   the column message_template of ai_connections (step 15);
//!         what earlier versions kept and this one no longer uses;
//!         names unique whatever their letter case
//! 2 → 3   the test projects and the evaluations of the model before
//!         OSCAL removed (step 41; the user decided: no migration)
//! 3 → 4   the settings of the scanner of serial ports removed (step 48:
//!         the program no longer works with devices)
//! ```
//!
//! TODO (step 10 of docs/todo.md): before the release, the steps that
//! only serve the databases of the development go; a released version
//! starts its own history of versions.

use rusqlite::Connection;

use crate::db::error::DbError;

/// The version of the layout this program writes.
pub const VERSION: i64 = 4;

/// Brings the database to [`VERSION`].
pub(super) fn run(connection: &Connection) -> Result<(), DbError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version >= VERSION {
        return Ok(());
    }

    let transaction = connection.unchecked_transaction()?;

    if version < 2 {
        to_2(&transaction)?;
    }

    if version < 3 {
        to_3(&transaction)?;
    }

    if version < 4 {
        to_4(&transaction)?;
    }

    transaction.execute_batch(&format!("PRAGMA user_version = {VERSION}"))?;
    transaction.commit()?;

    Ok(())
}

/// Version 2.
fn to_2(connection: &Connection) -> Result<(), DbError> {
    // A table ai_connections made before the column existed (the same
    // day, during step 15).
    let has_template: bool = connection.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('ai_connections') WHERE name = 'message_template'",
        [],
        |row| row.get(0),
    )?;

    if !has_template {
        connection.execute_batch(
            "ALTER TABLE ai_connections ADD COLUMN message_template TEXT NOT NULL \
             DEFAULT '{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}'",
        )?;
    }

    // What earlier versions kept and this one no longer uses: the API
    // key of Anthropic and the chosen Claude model, replaced by the AI
    // connections (step 15); the instructions of the analysis kept as
    // settings, replaced by db/ai/instructions/ (step 38; the user
    // agreed that they go back to the default ones). The key is
    // overwritten in the file.
    connection.execute_batch(
        "PRAGMA secure_delete = ON;
         DELETE FROM secrets WHERE name = 'anthropic.api_key';
         DELETE FROM settings WHERE name = 'ai.model';
         DELETE FROM settings WHERE name IN ('evaluation.analysis_instructions', 'evaluation.contents_instructions');",
    )?;

    // Names unique whatever their letter case, as the program checks
    // them: kept by the database itself. (An index, because the
    // collation of an existing column cannot be changed.)
    connection.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS ai_connections_name_nocase ON ai_connections (name COLLATE NOCASE);",
    )?;

    Ok(())
}

/// Version 3: the tables of the test projects and of the evaluations,
/// replaced by the documents of OSCAL (step 41). Their content is
/// overwritten in the file: an evaluation may hold what a manufacturer
/// gave under a non-disclosure agreement.
fn to_3(connection: &Connection) -> Result<(), DbError> {
    connection.execute_batch(
        "PRAGMA secure_delete = ON;
         DROP TRIGGER IF EXISTS evaluation_journal_unchanged;
         DROP TRIGGER IF EXISTS evaluation_journal_kept;
         DROP TABLE IF EXISTS evaluation_journal;
         DROP TABLE IF EXISTS evaluation_proposals;
         DROP TABLE IF EXISTS evaluation_results;
         DROP TABLE IF EXISTS evaluation_links;
         DROP TABLE IF EXISTS evaluation_tile_sources;
         DROP TABLE IF EXISTS evaluation_tiles;
         DROP TABLE IF EXISTS evaluation_schemes;
         DROP TABLE IF EXISTS evaluation_documents;
         DROP TABLE IF EXISTS evaluations;
         DROP TABLE IF EXISTS links;
         DROP TABLE IF EXISTS tile_sources;
         DROP TABLE IF EXISTS tiles;
         DROP TABLE IF EXISTS schemes;
         DROP TABLE IF EXISTS project_documents;
         DROP TABLE IF EXISTS projects;",
    )?;

    Ok(())
}

/// Version 4: the settings of the scanner of serial ports, whose view
/// was removed when the program left the work with devices (step 48).
fn to_4(connection: &Connection) -> Result<(), DbError> {
    connection.execute_batch("DELETE FROM settings WHERE name IN ('scan.port_types', 'scan.columns');")?;

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
    fn a_new_database_is_at_the_latest_version() {
        let directory = TempDir::new();
        let (db, _) = database(&directory);

        assert_eq!(version(&db), VERSION);
    }

    #[test]
    fn version_2_removes_what_earlier_versions_kept() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        crate::db::user::update::set_secret(&db, "anthropic.api_key", "sk-old").unwrap();
        crate::db::user::update::set_setting(&db, "ai.model", "claude-opus-5-5").unwrap();
        db.connection_for_tests().execute_batch("PRAGMA user_version = 1").unwrap();
        drop(db);

        let db = user_db::open(directory.path(), 1, &key).unwrap();
        assert!(crate::db::user::read::secret(&db, "anthropic.api_key").unwrap().is_none());
        assert!(crate::db::user::read::setting(&db, "ai.model").unwrap().is_none());
        assert_eq!(version(&db), VERSION);
    }

    #[test]
    fn version_2_adds_the_message_template() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        db.connection_for_tests()
            .execute_batch("ALTER TABLE ai_connections DROP COLUMN message_template; PRAGMA user_version = 1;")
            .unwrap();
        drop(db);

        let db = user_db::open(directory.path(), 1, &key).unwrap();
        let has_template: bool = db
            .connection_for_tests()
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('ai_connections') WHERE name = 'message_template'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_template);
    }

    #[test]
    fn names_of_ai_connections_are_unique_whatever_their_letter_case() {
        let directory = TempDir::new();
        let (db, _) = database(&directory);
        let index: i64 = db
            .connection_for_tests()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'ai_connections_name_nocase'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(index, 1);
    }

    #[test]
    fn version_3_removes_the_tables_of_the_model_before_oscal() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        db.connection_for_tests()
            .execute_batch("CREATE TABLE projects (id INTEGER PRIMARY KEY); PRAGMA user_version = 2;")
            .unwrap();
        drop(db);

        let db = user_db::open(directory.path(), 1, &key).unwrap();
        let left: i64 = db
            .connection_for_tests()
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name = 'projects'", [], |row| row.get(0))
            .unwrap();

        assert_eq!(left, 0);
        assert_eq!(version(&db), VERSION);
    }

    #[test]
    fn version_4_removes_the_settings_of_the_scanner() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        crate::db::user::update::set_setting(&db, "scan.port_types", "usb").unwrap();
        crate::db::user::update::set_setting(&db, "scan.columns", "path").unwrap();
        db.connection_for_tests().execute_batch("PRAGMA user_version = 3").unwrap();
        drop(db);

        let db = user_db::open(directory.path(), 1, &key).unwrap();
        assert!(crate::db::user::read::setting(&db, "scan.port_types").unwrap().is_none());
        assert!(crate::db::user::read::setting(&db, "scan.columns").unwrap().is_none());
        assert_eq!(version(&db), VERSION);
    }

    #[test]
    fn a_database_at_the_latest_version_runs_nothing() {
        let directory = TempDir::new();
        let (db, key) = database(&directory);

        // Something that version 2 would remove: kept, since it does not run.
        crate::db::user::update::set_setting(&db, "ai.model", "kept").unwrap();
        drop(db);

        let db = user_db::open(directory.path(), 1, &key).unwrap();
        assert_eq!(crate::db::user::read::setting(&db, "ai.model").unwrap().as_deref(), Some("kept"));
    }
}
