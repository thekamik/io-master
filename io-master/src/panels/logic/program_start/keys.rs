//! The keys of the databases of the evaluators
//! (`docs/architecture.md`, section 5.2).
//!
//! ```text
//! password ──Argon2id──HKDF─► unlocking key ──AES-256-GCM──► key of an evaluator's
//!                                                            database (random)
//! ```
//!
//! - The database of an **evaluator** has a random key. That key is
//!   stored only encrypted ([`WrappedKey`]) with an unlocking key.
//!   The password itself is never stored: a wrong password simply
//!   fails to decrypt (GCM detects it).
//! - **The password is the only factor** (the TPM factor of the earlier
//!   program was removed). HKDF after Argon2id stays, so that a further
//!   factor can be added to the unlocking key later.
//! - Changing the password encrypts the same random key again; the
//!   database itself is not touched.
//!
//! Argon2id turns the password into a key slowly and with a lot of
//! memory (64 MiB), so that trying many passwords is expensive,
//! also for someone who copied the files.

use std::fmt;

use aes_gcm::aead::{AeadInOut, Key, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::model::user::user_keys::WrappedKey;
use crate::security::random::fill_random;

use crate::db::encryption::database_key::{DATABASE_KEY_LENGTH, DatabaseKey};

/// The fewest characters of a password.
pub const PASSWORD_MIN: usize = 12;

/// The most characters of a password; a limit keeps a mistaken
/// paste of a whole document out.
pub const PASSWORD_MAX: usize = 1024;

/// Memory of Argon2id in KiB: 64 MiB, as recommended by OWASP.
/// The tests use little, to stay fast.
#[cfg(not(test))]
const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
#[cfg(test)]
const ARGON2_MEMORY_KIB: u32 = 256;

/// Passes of Argon2id over the memory.
#[cfg(not(test))]
const ARGON2_ITERATIONS: u32 = 3;
#[cfg(test)]
const ARGON2_ITERATIONS: u32 = 1;

/// What the HKDF derivation is for. A different text gives an
/// unrelated key, so a key made for one purpose never serves
/// another.
const UNLOCKING_KEY_PURPOSE: &[u8] = b"io-master unlocking key v1";

/// Length of the random salt of Argon2id.
const SALT_LENGTH: usize = 16;

/// Length of the nonce of AES-GCM.
const NONCE_LENGTH: usize = 12;

/// Length of the authentication tag of AES-GCM.
const TAG_LENGTH: usize = 16;

/// Why a key could not be made or opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// The password does not follow the rules; holds the reason.
    WeakPassword(String),
    /// The password does not open the key.
    WrongPassword,
    /// A stored key does not have the expected form.
    Damaged,
    /// No random bytes or no key could be made; holds the reason.
    Failed(String),
}

impl fmt::Display for KeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::WeakPassword(reason) => write!(formatter, "the password is too weak: {reason}"),
            KeyError::WrongPassword => formatter.write_str("wrong password"),
            KeyError::Damaged => formatter.write_str("a stored key is damaged"),
            KeyError::Failed(reason) => write!(formatter, "no key could be made: {reason}"),
        }
    }
}

impl std::error::Error for KeyError {}

/// Checks the rules of a password: 12 to 1024 characters. Nothing
/// else is demanded, as NIST recommends: length protects better
/// than forced special characters.
pub fn check_password(password: &str) -> Result<(), KeyError> {
    let length = password.chars().count();

    if length < PASSWORD_MIN {
        return Err(KeyError::WeakPassword(format!("a password has at least {PASSWORD_MIN} characters")));
    }

    if length > PASSWORD_MAX {
        return Err(KeyError::WeakPassword(format!("a password has at most {PASSWORD_MAX} characters")));
    }

    Ok(())
}

/// Derives the unlocking key from a password.
fn unlocking_key(
    password: &str,
    salt: &[u8],
    memory_kib: u32,
    iterations: u32,
) -> Result<Zeroizing<[u8; 32]>, KeyError> {
    let params =
        Params::new(memory_kib, iterations, 1, Some(32)).map_err(|error| KeyError::Failed(error.to_string()))?;

    // Argon2id of the password, then through HKDF (the place where a
    // further factor would join it).
    let mut stretched = Zeroizing::new([0u8; 32]);

    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut *stretched)
        .map_err(|error| KeyError::Failed(error.to_string()))?;

    let mut key = Zeroizing::new([0u8; 32]);

    Hkdf::<Sha256>::new(Some(salt), &*stretched)
        .expand(UNLOCKING_KEY_PURPOSE, &mut *key)
        .map_err(|error| KeyError::Failed(error.to_string()))?;

    Ok(key)
}

/// Encrypts the key of a database with a password.
///
/// `binding` ties the result to one account (for example its
/// name): an encrypted key copied to another account does not open
/// there.
pub fn wrap(key: &DatabaseKey, password: &str, binding: &[u8]) -> Result<WrappedKey, KeyError> {
    check_password(password)?;

    let mut salt = vec![0u8; SALT_LENGTH];
    let mut nonce = [0u8; NONCE_LENGTH];

    fill_random(&mut salt).map_err(|error| KeyError::Failed(error.to_string()))?;
    fill_random(&mut nonce).map_err(|error| KeyError::Failed(error.to_string()))?;

    let unlocking = unlocking_key(password, &salt, ARGON2_MEMORY_KIB, ARGON2_ITERATIONS)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*unlocking));

    // Encrypted in place: the copy that becomes the ciphertext.
    let mut buffer = key.expose().to_vec();

    let tag = cipher
        .encrypt_inout_detached(&Nonce::from(nonce), binding, buffer.as_mut_slice().into())
        .map_err(|error| KeyError::Failed(error.to_string()))?;

    Ok(WrappedKey {
        salt,
        memory_kib: ARGON2_MEMORY_KIB,
        iterations: ARGON2_ITERATIONS,
        nonce: nonce.to_vec(),
        ciphertext: buffer,
        tag: tag.to_vec(),
    })
}

/// Opens the key of a database with the password. Fails with
/// [`KeyError::WrongPassword`] when it does not fit, or when `binding`
/// is not the one used by [`wrap`].
pub fn unwrap(wrapped: &WrappedKey, password: &str, binding: &[u8]) -> Result<DatabaseKey, KeyError> {
    let nonce: [u8; NONCE_LENGTH] = wrapped.nonce.as_slice().try_into().map_err(|_| KeyError::Damaged)?;
    let tag: [u8; TAG_LENGTH] = wrapped.tag.as_slice().try_into().map_err(|_| KeyError::Damaged)?;

    if wrapped.ciphertext.len() != DATABASE_KEY_LENGTH || wrapped.salt.len() != SALT_LENGTH {
        return Err(KeyError::Damaged);
    }

    let unlocking = unlocking_key(password, &wrapped.salt, wrapped.memory_kib, wrapped.iterations)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*unlocking));

    // Decrypted straight into the memory of the new key.
    DatabaseKey::fill_with(|bytes| {
        bytes.copy_from_slice(&wrapped.ciphertext);

        cipher
            .decrypt_inout_detached(&Nonce::from(nonce), binding, bytes.as_mut_slice().into(), &tag.into())
            .map_err(|_| KeyError::WrongPassword)
    })
}

#[cfg(test)]
mod tests {
    //! Unit tests of `keys.rs`.

    use super::*;

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
        let wrapped = wrap(&database_key(), PASSWORD, b"anna").unwrap();

        assert_eq!(unwrap(&wrapped, PASSWORD, b"anna").unwrap().expose(), &[0x42; 32]);

        // The stored form does not contain the key.
        assert_ne!(wrapped.ciphertext, vec![0x42; 32]);
    }

    #[test]
    fn a_wrong_password_does_not_open_the_key() {
        let wrapped = wrap(&database_key(), PASSWORD, b"anna").unwrap();

        assert_eq!(unwrap(&wrapped, "wrong horse battery", b"anna").unwrap_err(), KeyError::WrongPassword);
    }

    #[test]
    fn a_key_of_one_account_does_not_open_for_another() {
        let wrapped = wrap(&database_key(), PASSWORD, b"anna").unwrap();

        assert_eq!(unwrap(&wrapped, PASSWORD, b"jan").unwrap_err(), KeyError::WrongPassword);
    }

    #[test]
    fn every_wrapping_is_different() {
        let first = wrap(&database_key(), PASSWORD, b"anna").unwrap();
        let second = wrap(&database_key(), PASSWORD, b"anna").unwrap();

        // New salt and nonce every time.
        assert_ne!(first.salt, second.salt);
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
    }

    #[test]
    fn a_damaged_wrapped_key_is_refused() {
        let mut wrapped = wrap(&database_key(), PASSWORD, b"anna").unwrap();

        wrapped.tag.pop();
        assert_eq!(unwrap(&wrapped, PASSWORD, b"anna").unwrap_err(), KeyError::Damaged);

        let mut changed = wrap(&database_key(), PASSWORD, b"anna").unwrap();
        changed.ciphertext[0] ^= 1;

        // A changed bit is detected, not turned into a wrong key.
        assert_eq!(unwrap(&changed, PASSWORD, b"anna").unwrap_err(), KeyError::WrongPassword);
    }

    #[test]
    fn passwords_follow_the_rules() {
        assert!(check_password("twelve chars").is_ok());
        assert!(check_password("ąęćśżźółńąęć").is_ok());
        assert!(matches!(check_password("eleven char"), Err(KeyError::WeakPassword(_))));
        assert!(matches!(check_password(&"x".repeat(PASSWORD_MAX + 1)), Err(KeyError::WeakPassword(_))));
        assert!(matches!(wrap(&database_key(), "short", b"anna"), Err(KeyError::WeakPassword(_))));
    }
}
