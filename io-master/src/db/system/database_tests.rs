//! Unit tests of `db/system/` (attached to `database.rs`).

use super::*;

use crate::db::encryption::keys::WrappedKey;
use crate::model::installation::{ID_LENGTH, Role};
use crate::tpm::test_support::TempDir;

fn installation() -> Installation {
    Installation { id: [5; ID_LENGTH], role: Role::Workstation }
}

fn some_key() -> WrappedKey {
    WrappedKey {
        salt: vec![1; 16],
        memory_kib: 256,
        iterations: 1,
        with_device_secret: false,
        nonce: vec![2; 12],
        ciphertext: vec![3; 32],
        tag: vec![4; 16],
    }
}

#[test]
fn the_installation_is_kept() {
    let directory = TempDir::new();

    SystemDb::create(directory.path(), &installation()).unwrap();

    let reopened = SystemDb::open(directory.path()).unwrap();
    assert_eq!(crate::db::system::read::installation(&reopened).unwrap(), installation());
}

#[test]
fn it_holds_no_secret_and_is_private() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDir::new();
    let db = SystemDb::create(directory.path(), &installation()).unwrap();

    crate::db::system::create::add_user(&db, "anna", &some_key()).unwrap();

    // A plain SQLite file, readable only by the user.
    let bytes = fs::read(path(directory.path())).unwrap();
    assert!(bytes.starts_with(b"SQLite format 3"));
    assert_eq!(fs::metadata(path(directory.path())).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn accounts_are_kept_with_their_keys() {
    let directory = TempDir::new();
    let db = SystemDb::create(directory.path(), &installation()).unwrap();

    let anna = crate::db::system::create::add_user(&db, "Anna", &some_key()).unwrap();
    crate::db::system::create::add_user(&db, "jan", &some_key()).unwrap();

    assert_eq!(crate::db::system::read::user_names(&db).unwrap(), ["Anna", "jan"]);

    // Found without regard to letter case.
    let found = crate::db::system::read::user(&db, "anna").unwrap().unwrap();
    assert_eq!((found.id, found.name.as_str(), &found.key), (anna, "Anna", &some_key()));
    assert_eq!(crate::db::system::read::user(&db, "nobody").unwrap(), None);
}

#[test]
fn a_name_is_taken_once_whatever_its_case() {
    let directory = TempDir::new();
    let db = SystemDb::create(directory.path(), &installation()).unwrap();

    crate::db::system::create::add_user(&db, "Anna", &some_key()).unwrap();

    assert!(crate::db::system::create::add_user(&db, "ANNA", &some_key()).is_err());
}

#[test]
fn a_key_can_be_replaced_and_an_account_removed() {
    let directory = TempDir::new();
    let db = SystemDb::create(directory.path(), &installation()).unwrap();
    let id = crate::db::system::create::add_user(&db, "anna", &some_key()).unwrap();

    let mut new_key = some_key();
    new_key.nonce = vec![9; 12];
    crate::db::system::update::replace_user_key(&db, id, &new_key).unwrap();
    assert_eq!(crate::db::system::read::user(&db, "anna").unwrap().unwrap().key, new_key);

    crate::db::system::delete::remove_user(&db, id).unwrap();
    assert_eq!(crate::db::system::read::user(&db, "anna").unwrap(), None);
    assert!(crate::db::system::update::replace_user_key(&db, id, &new_key).is_err());
}

#[test]
fn removing_the_database_leaves_nothing() {
    let directory = TempDir::new();

    SystemDb::create(directory.path(), &installation()).unwrap();
    SystemDb::remove(directory.path()).unwrap();

    assert!(!path(directory.path()).exists());
    SystemDb::remove(directory.path()).unwrap();
}
