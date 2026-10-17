//! Unit tests of `sqlcipher.rs`.

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

    create(&path, &key).unwrap().execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('kept');").unwrap();

    let value: String = open(&path, &key).unwrap().query_row("SELECT v FROM t", [], |row| row.get(0)).unwrap();

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

/// The value of a `PRAGMA` of SQLCipher.
fn pragma(connection: &Connection, name: &str) -> String {
    connection.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0)).unwrap()
}

/// The parameters of the encryption in effect, as the head of
/// `sqlcipher.rs` describes them. The program does not set them: they
/// are the defaults of SQLCipher 4. A newer SQLCipher that changed a
/// default fails here instead of changing the encryption unnoticed.
#[test]
fn the_parameters_of_the_encryption_are_those_described() {
    let directory = TempDir::new();
    let connection = create(&directory.path().join("p.db"), &key()).unwrap();

    assert!(pragma(&connection, "cipher_version").starts_with("4."), "{}", pragma(&connection, "cipher_version"));
    assert_eq!(pragma(&connection, "cipher_provider"), "openssl");
    assert_eq!(pragma(&connection, "cipher_page_size"), "4096");
    assert_eq!(pragma(&connection, "cipher_use_hmac"), "1");
    assert_eq!(pragma(&connection, "cipher_hmac_algorithm"), "HMAC_SHA512");
    assert_eq!(pragma(&connection, "cipher_plaintext_header_size"), "0");

    // Not used: the key is given as raw bytes, so SQLCipher derives
    // nothing from it (see open()).
    assert_eq!(pragma(&connection, "cipher_kdf_algorithm"), "PBKDF2_HMAC_SHA512");
    assert_eq!(pragma(&connection, "kdf_iter"), "256000");

    // Off: SQLCipher does not lock and wipe its own memory.
    assert_eq!(pragma(&connection, "cipher_memory_security"), "0");
}
