//! Unit tests of `accounts.rs`, with the password as the only
//! factor, as the program runs by default.

use super::*;

use crate::model::installation::{Installation, Role};
use crate::tpm::test_support::TempDir;

const PASSWORD: &str = "correct horse battery";

/// A system database of the given role in a temporary directory.
/// The third value is the secret of the device: none, as by
/// default.
fn setup(role: Role) -> (TempDir, SystemDb, Option<&'static DeviceSecret>) {
    let directory = TempDir::new();
    let installation = Installation { id: [1; 16], role };
    let db = SystemDb::create(directory.path(), &installation).unwrap();

    (directory, db, None)
}

#[test]
fn a_created_account_logs_in_and_opens_its_database() {
    let (directory, db, device) = setup(Role::Workstation);

    let created = create_first(&db, device, directory.path(), "Anna", PASSWORD).unwrap();
    let (account, key) = log_in(&db, device, directory.path(), "anna", PASSWORD).unwrap();

    // The name as it was given, whatever was typed at login.
    assert_eq!(account, created);
    assert_eq!(account.name, "Anna");
    assert!(user_db::open(directory.path(), account.id, &key).is_ok());
}

#[test]
fn a_wrong_password_and_an_unknown_name_look_the_same() {
    let (directory, db, device) = setup(Role::Workstation);

    create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();

    assert!(matches!(
        log_in(&db, device, directory.path(), "anna", "wrong horse battery"),
        Err(AccountError::WrongCredentials)
    ));
    assert!(matches!(log_in(&db, device, directory.path(), "nobody", PASSWORD), Err(AccountError::WrongCredentials)));
}

#[test]
fn every_account_has_its_own_database_and_key() {
    let (directory, db, device) = setup(Role::Workstation);

    create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();
    add(&db, device, directory.path(), "jan", "another good password").unwrap();

    let (anna, anna_key) = log_in(&db, device, directory.path(), "anna", PASSWORD).unwrap();
    let (jan, jan_key) = log_in(&db, device, directory.path(), "jan", "another good password").unwrap();

    assert_ne!(anna_key.expose(), jan_key.expose());

    // The key of one account does not open the database of another.
    assert!(user_db::open(directory.path(), jan.id, &anna_key).is_err());
    assert!(user_db::open(directory.path(), anna.id, &jan_key).is_err());
}

#[test]
fn a_master_keeps_its_one_user() {
    let (directory, db, device) = setup(Role::Master);

    create_first(&db, device, directory.path(), "admin", PASSWORD).unwrap();

    assert!(matches!(add(&db, device, directory.path(), "mallory", PASSWORD), Err(AccountError::NotAllowed)));
}

#[test]
fn names_and_passwords_are_checked() {
    let (directory, db, device) = setup(Role::Workstation);

    create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();

    assert!(matches!(add(&db, device, directory.path(), "ANNA", PASSWORD), Err(AccountError::NameTaken)));
    assert!(matches!(add(&db, device, directory.path(), " ", PASSWORD), Err(AccountError::InvalidName(_))));
    assert!(matches!(add(&db, device, directory.path(), "jan", "short"), Err(AccountError::WeakPassword(_))));

    // Refused accounts leave nothing behind.
    assert_eq!(db.user_names().unwrap(), ["anna"]);
}

#[test]
fn a_changed_password_opens_the_same_database() {
    let (directory, db, device) = setup(Role::Workstation);

    create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();
    let (_, before) = log_in(&db, device, directory.path(), "anna", PASSWORD).unwrap();

    change_password(&db, device, "anna", PASSWORD, "a brand new password").unwrap();

    let (_, after) = log_in(&db, device, directory.path(), "anna", "a brand new password").unwrap();
    assert_eq!(before.expose(), after.expose());
    assert!(log_in(&db, device, directory.path(), "anna", PASSWORD).is_err());
}

#[test]
fn a_password_is_changed_only_with_the_current_one() {
    let (directory, db, device) = setup(Role::Workstation);

    create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();

    assert!(matches!(
        change_password(&db, device, "anna", "not the password", "a brand new password"),
        Err(AccountError::WrongCredentials)
    ));
    assert!(matches!(change_password(&db, device, "anna", PASSWORD, "short"), Err(AccountError::WeakPassword(_))));
    assert!(log_in(&db, device, directory.path(), "anna", PASSWORD).is_ok());
}

#[test]
fn an_account_can_use_the_device_as_a_further_factor() {
    let (directory, db, _) = setup(Role::Workstation);
    let device = DeviceSecret::from_bytes([7; 32]);

    create_first(&db, Some(&device), directory.path(), "anna", PASSWORD).unwrap();

    assert!(log_in(&db, Some(&device), directory.path(), "anna", PASSWORD).is_ok());
    assert!(matches!(
        log_in(&db, None, directory.path(), "anna", PASSWORD),
        Err(AccountError::Key(KeyError::DeviceSecretMissing))
    ));
}
