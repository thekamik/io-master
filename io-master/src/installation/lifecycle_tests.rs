//! Unit tests of `lifecycle.rs`: the first start, the next start
//! and the removal, with passwords only (no TPM).

use super::*;
use crate::db::system;

use crate::test_support::TempDir;

const PASSWORD: &str = "correct horse battery";

/// Installs a workstation with the user "anna".
fn install_workstation(directory: &TempDir) -> Running {
    install(directory.path(), Role::Workstation, "anna", PASSWORD).unwrap().0
}

#[test]
fn no_system_database_is_the_first_start() {
    let directory = TempDir::new();

    assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));
}

#[test]
fn an_installation_opens_at_the_next_start() {
    let directory = TempDir::new();
    let installed = install_workstation(&directory);

    let Opened::Installed(running) = open(directory.path()).unwrap() else {
        panic!("the installation was not found");
    };

    assert_eq!(running.installation, installed.installation);
    assert_eq!(system::read::user_names(&running.system_db).unwrap(), ["anna"]);
    assert!(running.device_secret.is_none());

    // The first account logs in with its password alone.
    assert!(accounts::log_in(&running.system_db, None, directory.path(), "anna", PASSWORD).is_ok());
}

#[test]
fn the_installation_is_made_once() {
    let directory = TempDir::new();
    install_workstation(&directory);

    assert!(matches!(
        install(directory.path(), Role::Master, "mallory", PASSWORD),
        Err(LifecycleError::AlreadyInstalled)
    ));

    let Opened::Installed(running) = open(directory.path()).unwrap() else {
        panic!("the installation was lost");
    };
    assert_eq!(running.installation.role, Role::Workstation);
}

#[test]
fn a_file_that_is_not_a_system_database_is_damage() {
    let directory = TempDir::new();

    fs::write(system_db::path(directory.path()), b"not a database at all, just text").unwrap();

    assert!(matches!(open(directory.path()), Err(LifecycleError::Damaged(_))));
}

#[test]
fn a_mistake_in_the_form_changes_nothing() {
    let directory = TempDir::new();

    for (name, password) in [(" ", PASSWORD), ("anna", "short")] {
        assert!(matches!(
            install(directory.path(), Role::Workstation, name, password),
            Err(LifecycleError::Account(_))
        ));
    }

    assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));
}

#[test]
fn uninstall_leaves_nothing_and_allows_a_new_first_start() {
    let directory = TempDir::new();
    install_workstation(&directory);

    // What an earlier version with the TPM may have left.
    fs::write(device_secret::path(directory.path()), b"sealed").unwrap();
    fs::create_dir_all(directory.path().join(EMULATOR_DIRECTORY)).unwrap();

    uninstall(directory.path()).unwrap();

    assert!(!system_db::path(directory.path()).exists());
    assert!(!directory.path().join(user_db::DIRECTORY).exists());
    assert!(!device_secret::path(directory.path()).exists());
    assert!(!directory.path().join(EMULATOR_DIRECTORY).exists());
    assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));

    // Removing again is not an error.
    uninstall(directory.path()).unwrap();
}
