//! Unit tests of `keys.rs`. The secret of the device is a fixed
//! test value, so no TPM is needed.

use super::*;

/// A secret of a device for the tests.
fn device() -> DeviceSecret {
    DeviceSecret::from_bytes([7; 32])
}

/// A database key with known bytes.
fn database_key() -> DatabaseKey {
    DatabaseKey::fill_with(|bytes| {
        bytes.fill(0x42);
        Ok::<(), ()>(())
    })
    .unwrap()
}

const PASSWORD: &str = "correct horse battery";

#[test]
fn a_wrapped_key_opens_with_its_password() {
    let wrapped = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();

    assert_eq!(unwrap(&wrapped, PASSWORD, Some(&device()), b"anna").unwrap().expose(), &[0x42; 32]);

    // The stored form does not contain the key.
    assert_ne!(wrapped.ciphertext, vec![0x42; 32]);
}

#[test]
fn a_wrong_password_does_not_open_the_key() {
    let wrapped = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();

    assert_eq!(unwrap(&wrapped, "wrong horse battery", Some(&device()), b"anna").unwrap_err(), KeyError::WrongPassword);
}

#[test]
fn another_device_does_not_open_the_key() {
    let wrapped = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();
    let other = DeviceSecret::from_bytes([8; 32]);

    assert_eq!(unwrap(&wrapped, PASSWORD, Some(&other), b"anna").unwrap_err(), KeyError::WrongPassword);
}

#[test]
fn a_key_of_one_account_does_not_open_for_another() {
    let wrapped = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();

    assert_eq!(unwrap(&wrapped, PASSWORD, Some(&device()), b"jan").unwrap_err(), KeyError::WrongPassword);
}

#[test]
fn every_wrapping_is_different() {
    let first = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();
    let second = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();

    // New salt and nonce every time.
    assert_ne!(first.salt, second.salt);
    assert_ne!(first.nonce, second.nonce);
    assert_ne!(first.ciphertext, second.ciphertext);
}

#[test]
fn a_damaged_wrapped_key_is_refused() {
    let mut wrapped = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();

    wrapped.tag.pop();
    assert_eq!(unwrap(&wrapped, PASSWORD, Some(&device()), b"anna").unwrap_err(), KeyError::Damaged);

    let mut changed = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();
    changed.ciphertext[0] ^= 1;

    // A changed bit is detected, not turned into a wrong key.
    assert_eq!(unwrap(&changed, PASSWORD, Some(&device()), b"anna").unwrap_err(), KeyError::WrongPassword);
}

#[test]
fn passwords_follow_the_rules() {
    assert!(check_password("twelve chars").is_ok());
    assert!(check_password("ąęćśżźółńąęć").is_ok());
    assert!(matches!(check_password("eleven char"), Err(KeyError::WeakPassword(_))));
    assert!(matches!(check_password(&"x".repeat(PASSWORD_MAX + 1)), Err(KeyError::WeakPassword(_))));
    assert!(matches!(wrap(&database_key(), "short", Some(&device()), b"anna"), Err(KeyError::WeakPassword(_))));
}

#[test]
fn the_password_alone_is_the_default_factor() {
    let wrapped = wrap(&database_key(), PASSWORD, None, b"anna").unwrap();

    assert!(!wrapped.with_device_secret);
    assert_eq!(unwrap(&wrapped, PASSWORD, None, b"anna").unwrap().expose(), &[0x42; 32]);
    assert_eq!(unwrap(&wrapped, "wrong horse battery", None, b"anna").unwrap_err(), KeyError::WrongPassword);
}

#[test]
fn a_key_with_the_device_factor_needs_the_device() {
    let wrapped = wrap(&database_key(), PASSWORD, Some(&device()), b"anna").unwrap();

    assert!(wrapped.with_device_secret);
    assert_eq!(unwrap(&wrapped, PASSWORD, None, b"anna").unwrap_err(), KeyError::DeviceSecretMissing);
}

#[test]
fn the_factors_give_different_keys() {
    // The same salt and password with and without the device: the
    // unlocking keys differ, so the device really adds a factor.
    let with = unlocking_key(PASSWORD, &[1; SALT_LENGTH], 256, 1, Some(&device())).unwrap();
    let without = unlocking_key(PASSWORD, &[1; SALT_LENGTH], 256, 1, None).unwrap();

    assert_ne!(*with, *without);
}
