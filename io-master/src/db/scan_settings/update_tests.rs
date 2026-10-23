//! Unit tests of `db/scan_settings/` (attached to `update.rs`; they cover
//! the whole folder).

use super::*;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::scan_settings::read::{self, StoredScanSettings};
use crate::db::user::database as user_db;
use crate::security::random::fill_random;
use crate::test_support::TempDir;

/// A new database of an evaluator in `directory`.
fn database(directory: &TempDir) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    user_db::create(directory.path(), 1, &key).unwrap()
}

#[test]
fn nothing_stored_reads_as_none() {
    let directory = TempDir::new();

    assert_eq!(read::texts(&database(&directory)).unwrap(), StoredScanSettings { port_types: None, columns: None });
}

#[test]
fn stored_texts_are_read_back_and_replaced() {
    let directory = TempDir::new();
    let db = database(&directory);

    texts(&db, "ttyACM,ttyUSB", "path,driver").unwrap();
    texts(&db, "all", "path").unwrap();

    let stored = read::texts(&db).unwrap();
    assert_eq!(stored.port_types.as_deref(), Some("all"));
    assert_eq!(stored.columns.as_deref(), Some("path"));
}

#[test]
fn texts_that_cannot_be_stored_are_refused_and_nothing_changes() {
    let directory = TempDir::new();
    let db = database(&directory);
    texts(&db, "all", "path").unwrap();

    assert!(matches!(texts(&db, "", "path"), Err(DbError::Inconsistent(_))));
    assert!(matches!(texts(&db, "all", &"x".repeat(MAX_TEXT + 1)), Err(DbError::Inconsistent(_))));

    assert_eq!(read::texts(&db).unwrap().columns.as_deref(), Some("path"));
}
