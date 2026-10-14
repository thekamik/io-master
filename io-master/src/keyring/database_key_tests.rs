//! Unit tests of `database_key.rs`.
//!
//! The keyring of the system is replaced by a store in memory,
//! so the tests never touch a real database key.

use super::*;

use std::cell::RefCell;
use std::collections::HashMap;

/// A [`SecretStore`] that keeps its entries in memory.
#[derive(Default)]
struct MemoryStore {
    entries: RefCell<HashMap<String, String>>,
}

impl SecretStore for MemoryStore {
    fn store(&self, name: &SecretName, secret: &Secret) -> Result<(), KeyringError> {
        self.entries
            .borrow_mut()
            .insert(name.as_str().to_owned(), secret.expose().to_owned());
        Ok(())
    }

    fn load(&self, name: &SecretName) -> Result<Secret, KeyringError> {
        let entries = self.entries.borrow();
        let text = entries.get(name.as_str()).ok_or(KeyringError::NotFound)?;

        Secret::new(text.clone())
    }

    fn delete(&self, name: &SecretName) -> Result<(), KeyringError> {
        self.entries
            .borrow_mut()
            .remove(name.as_str())
            .map(|_| ())
            .ok_or(KeyringError::NotFound)
    }

    fn contains(&self, name: &SecretName) -> Result<bool, KeyringError> {
        Ok(self.entries.borrow().contains_key(name.as_str()))
    }
}

/// A key whose bytes are 0, 1, 2, ... 31.
fn counting_key() -> DatabaseKey {
    DatabaseKey::from_bytes(std::array::from_fn(|index| index as u8))
}

#[test]
fn a_key_is_kept_as_64_hexadecimal_digits() {
    let store = MemoryStore::default();

    store_database_key(&store, &counting_key()).unwrap();

    let stored = store.entries.borrow()[DATABASE_KEY_NAME].clone();

    assert_eq!(stored.len(), 64);
    assert!(stored.starts_with("000102030405"));
    assert!(stored.ends_with("1d1e1f"));
}

#[test]
fn a_stored_key_is_read_back_unchanged() {
    let store = MemoryStore::default();

    store_database_key(&store, &counting_key()).unwrap();

    assert_eq!(load_database_key(&store).unwrap().expose(), counting_key().expose());
}

#[test]
fn a_stored_key_is_never_replaced() {
    let store = MemoryStore::default();

    store_database_key(&store, &counting_key()).unwrap();

    let other = DatabaseKey::from_bytes([0xff; DATABASE_KEY_LENGTH]);

    assert_eq!(store_database_key(&store, &other).unwrap_err(), KeyringError::AlreadyExists);
    assert_eq!(load_database_key(&store).unwrap().expose(), counting_key().expose());
}

#[test]
fn a_missing_key_is_reported() {
    assert_eq!(load_database_key(&MemoryStore::default()).unwrap_err(), KeyringError::NotFound);
}

#[test]
fn a_damaged_entry_is_not_taken_for_a_key() {
    let store = MemoryStore::default();

    for text in ["abc", &"zz".repeat(32), &"00".repeat(33)] {
        store.entries.borrow_mut().insert(DATABASE_KEY_NAME.to_owned(), text.to_owned());

        assert!(
            matches!(load_database_key(&store), Err(KeyringError::Failed(_))),
            "{text:?} was taken for a key"
        );
    }
}

#[test]
fn hexadecimal_digits_of_either_case_are_read() {
    let upper = "AB".repeat(DATABASE_KEY_LENGTH);

    assert_eq!(
        DatabaseKey::from_hex(&upper).unwrap().expose(),
        &[0xab; DATABASE_KEY_LENGTH]
    );
}

#[test]
fn a_key_is_never_shown_in_debug_output() {
    let shown = format!("{:?}", DatabaseKey::from_bytes([0x5a; DATABASE_KEY_LENGTH]));

    assert_eq!(shown, "DatabaseKey(<hidden>)");
}
