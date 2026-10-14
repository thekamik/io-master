//! The key that encrypts the database of the program.
//!
//! The database itself comes later; this file only keeps its key
//! safe. The key is 32 random bytes (256 bits), the size that
//! SQLCipher and AES-256 use.
//!
//! The key lives in the keyring of the system, under the name
//! [`DATABASE_KEY_NAME`], next to the API keys. The keyring keeps
//! text, so the bytes are stored as 64 hexadecimal digits.
//!
//! Two rules differ from other secrets:
//!
//! - **A stored key is never replaced.** The database can only be
//!   opened with the key it was encrypted with. Replacing the key
//!   would make the data unreadable for good, so
//!   [`store_database_key`] refuses when a key is already there.
//!   Changing the key will be a separate operation of the
//!   database: re-encrypt first, then store the new key.
//! - **The key is binary.** [`DatabaseKey`] holds exactly 32
//!   bytes, so a key of the wrong length cannot exist.
//!
//! Generating the key belongs to the database
//! (`db/generate_key.rs`); this file accepts the bytes from
//! wherever they come.

// TODO: Remove when the database uses this file; until then the
//       compiler sees no user of it.
#![allow(dead_code)]

use std::fmt;

use zeroize::Zeroizing;

use super::keyring::{KeyringError, Secret, SecretName, SecretStore};

/// Name of the entry in the keyring.
pub const DATABASE_KEY_NAME: &str = "database-key";

/// Length of the key in bytes.
pub const DATABASE_KEY_LENGTH: usize = 32;

/// The key of the database.
///
/// Like [`Secret`], it is wiped from memory when dropped, is not
/// shown by `{:?}`, cannot be cloned, and gives its bytes out
/// only through [`DatabaseKey::expose`].
///
/// The bytes live on the heap (`Box`). Handing a `DatabaseKey`
/// from one function to another then moves only a pointer; bytes
/// kept in the value itself would be copied on every move, and
/// those copies would not be wiped.
pub struct DatabaseKey(Box<Zeroizing<[u8; DATABASE_KEY_LENGTH]>>);

impl DatabaseKey {
    /// A key of zeros, to be filled in place.
    fn zeroed() -> DatabaseKey {
        DatabaseKey(Box::new(Zeroizing::new([0; DATABASE_KEY_LENGTH])))
    }

    /// Creates a key by letting `fill` write its bytes directly
    /// into the memory of the key, so they exist only there.
    /// When `fill` fails, its error is returned and the partly
    /// written bytes are wiped.
    pub fn fill_with<E>(
        fill: impl FnOnce(&mut [u8; DATABASE_KEY_LENGTH]) -> Result<(), E>,
    ) -> Result<DatabaseKey, E> {
        let mut key = DatabaseKey::zeroed();

        fill(&mut key.0)?;

        Ok(key)
    }

    /// Takes the bytes of a key.
    ///
    /// The array passed in is a copy the caller has to take care
    /// of; prefer [`DatabaseKey::fill_with`] for real keys.
    pub fn from_bytes(bytes: [u8; DATABASE_KEY_LENGTH]) -> DatabaseKey {
        let mut key = DatabaseKey::zeroed();

        **key.0 = bytes;

        key
    }

    /// The bytes of the key. Use them only where the key is
    /// actually needed: when the database is opened.
    pub fn expose(&self) -> &[u8; DATABASE_KEY_LENGTH] {
        &self.0
    }

    /// The key as 64 hexadecimal digits, the form kept in the
    /// keyring.
    fn to_hex(&self) -> Zeroizing<String> {
        let mut text = Zeroizing::new(String::with_capacity(DATABASE_KEY_LENGTH * 2));

        for byte in self.0.iter() {
            // Two digits per byte, written without a temporary
            // string, so no unwiped copy of the key is made.
            text.push(hex_digit(byte >> 4));
            text.push(hex_digit(byte & 0x0f));
        }

        text
    }

    /// The key from its hexadecimal form. `None` when the text
    /// is not exactly 64 hexadecimal digits.
    fn from_hex(text: &str) -> Option<DatabaseKey> {
        if text.len() != DATABASE_KEY_LENGTH * 2 {
            return None;
        }

        let mut key = DatabaseKey::zeroed();

        for (byte, pair) in key.0.iter_mut().zip(text.as_bytes().chunks(2)) {
            *byte = (hex_value(pair[0])? << 4) | hex_value(pair[1])?;
        }

        Some(key)
    }
}

impl fmt::Debug for DatabaseKey {
    /// Shows that there is a key, never what it is.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DatabaseKey(<hidden>)")
    }
}

/// The hexadecimal digit of a value from 0 to 15.
fn hex_digit(value: u8) -> char {
    char::from(b"0123456789abcdef"[usize::from(value)])
}

/// The value of a hexadecimal digit, either letter case.
fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// The name of the entry, as a [`SecretName`].
fn entry_name() -> SecretName {
    SecretName::new(DATABASE_KEY_NAME).expect("the name of the database key is valid")
}

/// Stores the key of the database in the keyring.
///
/// Fails with [`KeyringError::AlreadyExists`] when a key is
/// already stored: see the module documentation for why a key is
/// never replaced.
pub fn store_database_key(
    store: &impl SecretStore,
    key: &DatabaseKey,
) -> Result<(), KeyringError> {
    let name = entry_name();

    if store.contains(&name)? {
        return Err(KeyringError::AlreadyExists);
    }

    // The hexadecimal text is moved out of its wrapper into the
    // Secret, so it exists once and is wiped when dropped.
    let mut text = key.to_hex();
    let secret = Secret::new(std::mem::take(&mut *text))?;

    store.store(&name, &secret)
}

/// Reads the key of the database from the keyring.
///
/// Fails with [`KeyringError::NotFound`] when no key is stored,
/// for example before the database was created, and with
/// [`KeyringError::Failed`] when the stored text is not a key.
pub fn load_database_key(store: &impl SecretStore) -> Result<DatabaseKey, KeyringError> {
    let secret = store.load(&entry_name())?;

    DatabaseKey::from_hex(secret.expose()).ok_or_else(|| {
        KeyringError::Failed(format!(
            "the entry '{DATABASE_KEY_NAME}' does not hold a database key"
        ))
    })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "database_key_tests.rs"]
mod tests;
