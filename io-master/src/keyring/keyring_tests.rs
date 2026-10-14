//! Unit tests of `keyring.rs`.
//!
//! The checks of names and secrets need nothing. The real keyring
//! is used only by the tests marked `#[ignore]`, which are run by
//! hand:
//!
//! ```text
//! cargo test keyring -- --ignored
//! ```

use super::*;

#[test]
fn valid_names_are_accepted() {
    for name in ["anthropic", "gemini", "api.key-2", "A_b"] {
        assert_eq!(SecretName::new(name).unwrap().as_str(), name);
    }
}

#[test]
fn invalid_names_are_refused() {
    let too_long = "n".repeat(NAME_MAX + 1);

    for name in ["", "with space", "slash/name", "ąę", "a:b", too_long.as_str()] {
        assert!(
            matches!(SecretName::new(name), Err(KeyringError::InvalidName(_))),
            "{name:?} was accepted"
        );
    }
}

#[test]
fn an_api_key_is_trimmed() {
    let key = Secret::api_key("  sk-ant-api03-abc_DEF-123\n".to_owned()).unwrap();

    assert_eq!(key.expose(), "sk-ant-api03-abc_DEF-123");
}

#[test]
fn an_api_key_must_be_one_printable_word() {
    for text in ["", "   \n", "two words", "tab\tinside", "zażółć"] {
        assert!(
            matches!(Secret::api_key(text.to_owned()), Err(KeyringError::InvalidSecret(_))),
            "{text:?} was accepted"
        );
    }
}

#[test]
fn a_secret_has_limits() {
    assert!(Secret::new(String::new()).is_err());
    assert!(Secret::new("x".repeat(SECRET_MAX + 1)).is_err());

    // A general secret is kept as it is, spaces and all.
    assert_eq!(Secret::new(" pass phrase ".to_owned()).unwrap().expose(), " pass phrase ");
}

#[test]
fn a_secret_is_never_shown_in_debug_output() {
    let key = Secret::api_key("sk-very-secret".to_owned()).unwrap();

    let shown = format!("{key:?} {:?}", Some(&key));

    assert!(!shown.contains("very-secret"));
    assert!(shown.contains("<hidden>"));
}

#[test]
fn keyring_errors_are_explained() {
    let error = KeyringError::from(secret_service::Error::Unavailable);

    assert_eq!(error, KeyringError::Unavailable);
    assert_eq!(KeyringError::from(secret_service::Error::Prompt), KeyringError::Dismissed);
    assert_eq!(error.to_string(), "the keyring of the system cannot be reached");
}

/// Stores, reads and removes an entry in the real keyring of this
/// computer. The entry is named `io-master-test` and is removed
/// at the end. When the keyring is locked, the system asks to
/// unlock it.
#[test]
#[ignore]
fn the_system_keyring_keeps_a_secret() {
    let keyring = SystemKeyring;
    let name = SecretName::new("io-master-test").unwrap();

    // A leftover of an interrupted earlier run does not matter.
    let _ = keyring.delete(&name);

    keyring.store(&name, &Secret::api_key("test-key-1".to_owned()).unwrap()).unwrap();
    assert!(keyring.contains(&name).unwrap());
    assert_eq!(keyring.load(&name).unwrap().expose(), "test-key-1");

    // Storing again replaces the secret.
    keyring.store(&name, &Secret::api_key("test-key-2".to_owned()).unwrap()).unwrap();
    assert_eq!(keyring.load(&name).unwrap().expose(), "test-key-2");

    keyring.delete(&name).unwrap();
    assert!(!keyring.contains(&name).unwrap());
    assert_eq!(keyring.load(&name).unwrap_err(), KeyringError::NotFound);
    assert_eq!(keyring.delete(&name).unwrap_err(), KeyringError::NotFound);
}
