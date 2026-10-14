//! Unit tests of `lifecycle.rs`, on the TPM emulator: every state
//! of the table in the module documentation.

use super::*;

use std::fs;

use crate::tpm::test_support::TestTpm;

/// Installs a workstation with the user "anna".
fn install_workstation(test: &TestTpm) -> Installation {
    install(test.dir(), &mut test.connect(), Role::Workstation, "anna").unwrap()
}

fn open_state(test: &TestTpm) -> Result<State, LifecycleError> {
    open(test.dir(), &mut test.connect())
}

#[test]
fn nothing_installed_is_the_first_start() {
    let test = TestTpm::new();

    assert_eq!(open_state(&test).unwrap(), State::FirstRun);
}

#[test]
fn an_installation_opens_at_the_next_start() {
    let test = TestTpm::new();
    let installed = install_workstation(&test);

    assert_eq!(open_state(&test).unwrap(), State::Installed(installed.clone()));
    assert_eq!(test.connect().read_marker().unwrap(), Some(installed.id));
}

#[test]
fn a_second_installation_is_refused() {
    let test = TestTpm::new();
    install_workstation(&test);

    // Another data directory on the same computer (the same TPM).
    let other = test.dir().join("other");
    fs::create_dir_all(&other).unwrap();

    assert!(matches!(
        install(&other, &mut test.connect(), Role::Master, "mallory"),
        Err(LifecycleError::AnotherInstallation)
    ));
}

#[test]
fn a_removed_record_is_damage() {
    let test = TestTpm::new();
    install_workstation(&test);

    installation::remove(test.dir()).unwrap();

    assert!(matches!(open_state(&test), Err(LifecycleError::Damaged(_))));
}

#[test]
fn a_missing_marker_is_damage() {
    let test = TestTpm::new();
    install_workstation(&test);

    // As if the TPM was cleared, or the files moved to another
    // computer.
    test.connect().remove_marker().unwrap();

    assert!(matches!(open_state(&test), Err(LifecycleError::Damaged(_))));
}

#[test]
fn a_record_of_another_installation_is_damage() {
    let test = TestTpm::new();
    install_workstation(&test);

    let mut tpm = test.connect();
    tpm.remove_marker().unwrap();
    tpm.write_marker(&[1; installation::ID_LENGTH]).unwrap();

    assert!(matches!(open_state(&test), Err(LifecycleError::Damaged(_))));
}

#[test]
fn an_interrupted_first_start_is_damage() {
    let test = TestTpm::new();

    // Only the first part was made before the program stopped.
    device_secret::create(&mut test.connect(), test.dir()).unwrap();

    assert!(matches!(open_state(&test), Err(LifecycleError::Damaged(_))));
}

#[test]
fn uninstall_leaves_nothing_and_allows_a_new_first_start() {
    let test = TestTpm::new();
    install_workstation(&test);

    uninstall(test.dir(), &mut test.connect()).unwrap();

    assert_eq!(test.connect().read_marker().unwrap(), None);
    assert!(!device_secret::path(test.dir()).exists());
    assert_eq!(open_state(&test).unwrap(), State::FirstRun);

    // A damaged installation can be removed as well.
    install_workstation(&test);
    installation::remove(test.dir()).unwrap();
    uninstall(test.dir(), &mut test.connect()).unwrap();
    assert_eq!(open_state(&test).unwrap(), State::FirstRun);
}

#[test]
fn an_invalid_first_user_changes_nothing() {
    let test = TestTpm::new();

    assert!(install(test.dir(), &mut test.connect(), Role::Workstation, " ").is_err());
    assert_eq!(test.connect().read_marker().unwrap(), None);
    assert_eq!(open_state(&test).unwrap(), State::FirstRun);
}
