//! Unit tests of `device_secret.rs`, on the TPM emulator.

use super::*;

use std::os::unix::fs::PermissionsExt;

use crate::security::tpm::test_support::TestTpm;

#[test]
fn the_secret_is_kept_sealed_and_opens_again() {
    let test = TestTpm::new();

    let created = create(&mut test.connect(), test.dir()).unwrap();
    let loaded = load(&mut test.connect(), test.dir()).unwrap();

    assert_eq!(created.expose(), loaded.expose());
    assert_ne!(created.expose(), &[0; LENGTH]);

    // The envelope does not contain the secret.
    let file = fs::read(path(test.dir())).unwrap();
    assert!(!file.windows(LENGTH).any(|window| window == created.expose()));

    let mode = fs::metadata(path(test.dir())).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn a_secret_is_never_replaced() {
    let test = TestTpm::new();

    let first = create(&mut test.connect(), test.dir()).unwrap();

    assert!(matches!(create(&mut test.connect(), test.dir()), Err(TpmError::Io { .. })));
    assert_eq!(load(&mut test.connect(), test.dir()).unwrap().expose(), first.expose());
}

#[test]
fn a_missing_or_damaged_secret_is_reported() {
    let test = TestTpm::new();

    assert!(matches!(load(&mut test.connect(), test.dir()), Err(TpmError::Io { .. })));

    fs::write(path(test.dir()), b"not an envelope").unwrap();
    assert!(load(&mut test.connect(), test.dir()).is_err());
}

#[test]
fn removing_the_secret_leaves_nothing() {
    let test = TestTpm::new();

    create(&mut test.connect(), test.dir()).unwrap();
    remove(test.dir()).unwrap();

    assert!(!path(test.dir()).exists());
    remove(test.dir()).unwrap();
}

#[test]
fn the_secret_is_never_shown_in_debug_output() {
    let test = TestTpm::new();
    let secret = create(&mut test.connect(), test.dir()).unwrap();

    assert_eq!(format!("{secret:?}"), "DeviceSecret(<hidden>)");
}
