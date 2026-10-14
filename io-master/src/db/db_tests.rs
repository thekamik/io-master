//! Unit tests of `db.rs`.

use super::*;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

/// A random key.
fn key() -> DatabaseKey {
    DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap()
}

#[test]
fn a_database_opens_again_with_its_key() {
    let directory = TempDir::new();
    let path = directory.path().join("test.db");
    let key = key();

    create(&path, &key)
        .unwrap()
        .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('kept');")
        .unwrap();

    let value: String = open(&path, &key)
        .unwrap()
        .query_row("SELECT v FROM t", [], |row| row.get(0))
        .unwrap();

    assert_eq!(value, "kept");
}

#[test]
fn the_file_is_encrypted_and_private() {
    let directory = TempDir::new();
    let path = directory.path().join("test.db");

    create(&path, &key())
        .unwrap()
        .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('a readable secret');")
        .unwrap();

    let bytes = fs::read(&path).unwrap();

    // A plain SQLite file starts with this text; an encrypted one
    // does not, and the stored text is not readable either.
    assert!(!bytes.starts_with(b"SQLite format 3"));
    assert!(!bytes.windows(17).any(|window| window == b"a readable secret"));

    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn another_key_does_not_open_the_database() {
    let directory = TempDir::new();
    let path = directory.path().join("test.db");

    create(&path, &key()).unwrap().execute_batch("CREATE TABLE t (v)").unwrap();

    assert!(matches!(open(&path, &key()), Err(DbError::WrongKey(_))));
}

#[test]
fn an_existing_database_is_never_created_again() {
    let directory = TempDir::new();
    let path = directory.path().join("test.db");

    create(&path, &key()).unwrap();

    assert!(matches!(create(&path, &key()), Err(DbError::Exists(_))));
    assert!(matches!(open(&directory.path().join("missing.db"), &key()), Err(DbError::NotFound(_))));
}

#[test]
fn a_new_empty_database_opens_only_with_its_key() {
    let directory = TempDir::new();
    let path = directory.path().join("empty.db");

    // No table is created: the database stays as create() made it.
    create(&path, &key()).unwrap();

    assert!(fs::metadata(&path).unwrap().len() > 0);
    assert!(matches!(open(&path, &key()), Err(DbError::WrongKey(_))));
}

#[test]
fn every_database_knows_its_schema_version() {
    let directory = TempDir::new();
    let path = directory.path().join("versioned.db");
    let key = key();

    create(&path, &key).unwrap();

    let version: i64 = open(&path, &key).unwrap().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, SCHEMA_VERSION);
}
