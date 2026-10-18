//! Unit tests of `verify.rs`, in temporary folders.

use super::*;

use std::fs;

use crate::model::installation::Role;
use crate::test_support::TempDir;

const PASSWORD: &str = "correct horse battery";

#[test]
fn without_a_location_the_installation_is_damaged() {
    assert!(matches!(verify_at(None), InstallationState::Damaged(Damage::NoLocation)));
}

#[test]
fn an_empty_folder_is_not_installed() {
    let directory = TempDir::new();

    let InstallationState::NotInstalled { data_dir } = verify_at(Some(directory.path().to_owned())) else {
        panic!("an empty folder must be NotInstalled");
    };

    assert_eq!(data_dir, directory.path());
}

#[test]
fn an_installation_is_found_and_opened() {
    let directory = TempDir::new();
    lifecycle::install(directory.path(), Role::Workstation, "anna", PASSWORD).unwrap();

    let InstallationState::Installed { data_dir, running } = verify_at(Some(directory.path().to_owned())) else {
        panic!("the installation was not found");
    };

    assert_eq!(data_dir, directory.path());
    assert_eq!(running.installation.role, Role::Workstation);
}

#[test]
fn a_system_database_that_cannot_be_read_is_damaged() {
    let directory = TempDir::new();
    fs::write(directory.path().join("system.db"), b"not a database").unwrap();

    let state = verify_at(Some(directory.path().to_owned()));

    let InstallationState::Damaged(damage @ Damage::Unreadable { .. }) = state else {
        panic!("a broken system database must be Damaged");
    };

    assert!(damage.to_string().contains("cannot be opened"));
}
