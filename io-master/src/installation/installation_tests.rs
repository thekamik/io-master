//! Unit tests of `installation.rs`.

use super::*;

use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A data directory for one test, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> TempDir {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        TempDir(std::env::temp_dir().join(format!(
            "io-master-installation-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_empty_data_directory_is_the_first_start() {
    let directory = TempDir::new();

    assert_eq!(load(&directory.0).unwrap(), State::FirstRun);
}

#[test]
fn the_choice_of_the_first_start_is_kept() {
    let directory = TempDir::new();

    let created = create(&directory.0, Role::Master, "admin").unwrap();

    assert_eq!(load(&directory.0).unwrap(), State::Installed(created.clone()));
    assert_eq!(created.users, ["admin"]);
    assert!(!created.can_add_users());
}

#[test]
fn the_installation_is_written_only_once() {
    let directory = TempDir::new();

    create(&directory.0, Role::Workstation, "anna").unwrap();

    assert!(matches!(
        create(&directory.0, Role::Master, "mallory"),
        Err(InstallationError::AlreadyInstalled)
    ));
    assert!(matches!(load(&directory.0).unwrap(), State::Installed(i) if i.role == Role::Workstation));
}

#[test]
fn the_files_are_private() {
    let directory = TempDir::new();

    create(&directory.0, Role::Workstation, "anna").unwrap();

    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;

    assert_eq!(mode(&directory.0), 0o700);
    assert_eq!(mode(&directory.0.join(FILE_NAME)), 0o600);
}

#[test]
fn user_names_are_checked() {
    for name in ["anna", "Jan Kowalski", "ewaluator-2"] {
        assert!(check_user_name(name).is_ok(), "{name}");
    }

    let too_long = "n".repeat(USER_NAME_MAX + 1);

    for name in ["", " anna", "anna ", "an\nna", too_long.as_str()] {
        assert!(check_user_name(name).is_err(), "{name:?}");
    }
}

#[test]
fn a_damaged_file_stops_the_program() {
    for text in [
        "not json",
        "{\"users\": [\"a\"]}",
        "{\"role\": \"workstation\", \"users\": []}",
        "{\"role\": \"master\", \"users\": [\"a\", \"b\"]}",
    ] {
        let directory = TempDir::new();

        fs::create_dir_all(&directory.0).unwrap();
        fs::write(directory.0.join(FILE_NAME), text).unwrap();

        assert!(
            matches!(load(&directory.0), Err(InstallationError::Damaged { .. })),
            "{text:?} was accepted"
        );
    }
}

#[test]
fn roles_have_names() {
    for role in [Role::Workstation, Role::Master] {
        assert_eq!(Role::from_name(role.name()), Some(role));
    }

    assert_eq!(Role::from_name("admin"), None);
}
