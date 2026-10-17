//! Unit tests of `db/user/` (attached to `database.rs`).

use super::*;

use std::os::unix::fs::PermissionsExt;

use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

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
fn all_databases_are_removed() {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    create(directory.path(), 1, &key).unwrap();
    create(directory.path(), 2, &key).unwrap();
    remove_all(directory.path()).unwrap();

    assert!(!directory.path().join(DIRECTORY).exists());
    remove_all(directory.path()).unwrap();
}

#[test]
fn settings_are_kept_and_replaced() {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    let db = create(directory.path(), 1, &key).unwrap();

    assert_eq!(db.setting("scan.columns").unwrap(), None);

    db.set_setting("scan.columns", "path,type").unwrap();
    db.set_setting("scan.columns", "path").unwrap();
    drop(db);

    // Kept in the file, and readable only with the key.
    let db = open(directory.path(), 1, &key).unwrap();
    assert_eq!(db.setting("scan.columns").unwrap().as_deref(), Some("path"));
}

#[test]
fn secrets_are_kept_replaced_and_removed() {
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let db = create(directory.path(), 1, &key).unwrap();

    assert_eq!(db.secret("api").unwrap(), None);

    db.set_secret("api", "first").unwrap();
    db.set_secret("api", "second").unwrap();
    assert_eq!(db.secret("api").unwrap().as_deref().map(String::as_str), Some("second"));

    // Secrets and settings do not mix.
    assert_eq!(db.setting("api").unwrap(), None);

    db.remove_secret("api").unwrap();
    db.remove_secret("api").unwrap();
    assert_eq!(db.secret("api").unwrap(), None);
}
