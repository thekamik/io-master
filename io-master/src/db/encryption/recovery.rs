//! The second copy of the key of a database, wrapped with the recovery
//! code (step 49 of docs/history.md, `docs/architecture.md`, section 5.4):
//! the way back to the data after a forgotten password, as the recovery
//! keys of BitLocker, FileVault and LUKS.
//!
//! ```text
//! recovery code (160 random bits) ──HKDF──► recovery key ──AES-256-GCM──► key of the database
//! ```
//!
//! Argon2id is not used here: it makes the guessing of a password slow,
//! and 160 random bits cannot be guessed at all. The copy is stored in
//! the system database beside the copy wrapped with the password
//! (`keys.rs`); neither tells anything without its secret.

use std::fmt;

use aes_gcm::aead::{AeadInOut, Key, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::model_old::recovery_code::{CODE_BYTES, RecoveryCode};
use crate::security::random::fill_random;

use super::database_key::{DATABASE_KEY_LENGTH, DatabaseKey};

/// What the HKDF derivation is for: never the same key as the one of the
/// password (`keys.rs`, "io-master unlocking key v1").
const RECOVERY_KEY_PURPOSE: &[u8] = b"io-master recovery key v1";

/// Length of the random salt of HKDF.
const SALT_LENGTH: usize = 16;

/// Length of the nonce of AES-GCM.
const NONCE_LENGTH: usize = 12;

/// Length of the authentication tag of AES-GCM.
const TAG_LENGTH: usize = 16;

/// Why a recovery copy could not be made or opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    /// The code does not open the key.
    WrongCode,
    /// A stored copy does not have the expected form.
    Damaged,
    /// No random bytes or no key could be made; holds the reason.
    Failed(String),
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryError::WrongCode => formatter.write_str("wrong recovery code"),
            RecoveryError::Damaged => formatter.write_str("a stored recovery key is damaged"),
            RecoveryError::Failed(reason) => write!(formatter, "no recovery key could be made: {reason}"),
        }
    }
}

impl std::error::Error for RecoveryError {}

/// The key of a database, wrapped with a recovery code. Everything
/// needed to open it again, except the code; none of it is secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryWrappedKey {
    /// The random salt of HKDF.
    pub salt: Vec<u8>,
    /// The nonce of AES-GCM: random, used once.
    pub nonce: Vec<u8>,
    /// The encrypted key and its authentication tag.
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
}

/// A new recovery code: 160 random bits from the kernel.
///
/// Called by: `installation/accounts.rs` (the account made, the password
/// changed or recovered, a new code asked for).
/// Calls: `security::random::fill_random`.
pub fn new_code() -> Result<RecoveryCode, RecoveryError> {
    let mut bytes = Zeroizing::new([0u8; CODE_BYTES]);

    fill_random(&mut *bytes).map_err(|error| RecoveryError::Failed(error.to_string()))?;

    Ok(RecoveryCode::from_bytes(*bytes))
}

/// The key that the code and the salt give.
///
/// Called by: [`wrap`], [`unwrap`].
fn recovery_key(code: &RecoveryCode, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, RecoveryError> {
    ensure!(salt.len() == SALT_LENGTH, RecoveryError::Damaged);

    let mut key = Zeroizing::new([0u8; 32]);

    Hkdf::<Sha256>::new(Some(salt), code.expose())
        .expand(RECOVERY_KEY_PURPOSE, &mut *key)
        .map_err(|error| RecoveryError::Failed(error.to_string()))?;

    Ok(key)
}

/// Wraps the key of a database with a recovery code. `binding` ties the
/// copy to its account, as in `keys::wrap`.
///
/// Called by: `installation/accounts.rs`.
/// Calls: `fill_random`, [`recovery_key`], AES-256-GCM.
pub fn wrap(key: &DatabaseKey, code: &RecoveryCode, binding: &[u8]) -> Result<RecoveryWrappedKey, RecoveryError> {
    ensure!(!binding.is_empty(), RecoveryError::Failed("a copy of a key needs its account".to_owned()));

    let mut salt = vec![0u8; SALT_LENGTH];
    let mut nonce = [0u8; NONCE_LENGTH];

    fill_random(&mut salt).map_err(|error| RecoveryError::Failed(error.to_string()))?;
    fill_random(&mut nonce).map_err(|error| RecoveryError::Failed(error.to_string()))?;

    let recovery = recovery_key(code, &salt)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*recovery));

    // Encrypted in place: the copy that becomes the ciphertext.
    let mut buffer = key.expose().to_vec();

    let tag = cipher
        .encrypt_inout_detached(&Nonce::from(nonce), binding, buffer.as_mut_slice().into())
        .map_err(|error| RecoveryError::Failed(error.to_string()))?;

    Ok(RecoveryWrappedKey { salt, nonce: nonce.to_vec(), ciphertext: buffer, tag: tag.to_vec() })
}

/// Opens the key of a database with a recovery code. Fails with
/// [`RecoveryError::WrongCode`] when the code (or `binding`) does not fit.
///
/// Called by: `installation/accounts.rs` (`recover`).
/// Calls: [`recovery_key`], AES-256-GCM.
pub fn unwrap(wrapped: &RecoveryWrappedKey, code: &RecoveryCode, binding: &[u8]) -> Result<DatabaseKey, RecoveryError> {
    let nonce: [u8; NONCE_LENGTH] = wrapped.nonce.as_slice().try_into().map_err(|_| RecoveryError::Damaged)?;
    let tag: [u8; TAG_LENGTH] = wrapped.tag.as_slice().try_into().map_err(|_| RecoveryError::Damaged)?;

    ensure!(wrapped.ciphertext.len() == DATABASE_KEY_LENGTH, RecoveryError::Damaged);

    let recovery = recovery_key(code, &wrapped.salt)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*recovery));

    // Decrypted straight into the memory of the new key.
    DatabaseKey::fill_with(|bytes| {
        bytes.copy_from_slice(&wrapped.ciphertext);

        cipher
            .decrypt_inout_detached(&Nonce::from(nonce), binding, bytes.as_mut_slice().into(), &tag.into())
            .map_err(|_| RecoveryError::WrongCode)
    })
}

#[cfg(test)]
mod tests {
    //! Unit tests of `recovery.rs`.

    use super::*;

    fn database_key() -> DatabaseKey {
        DatabaseKey::fill_with(|bytes| {
            bytes.fill(0x42);
            Ok::<(), ()>(())
        })
        .unwrap()
    }

    #[test]
    fn a_wrapped_key_opens_with_its_code() {
        let code = new_code().unwrap();
        let wrapped = wrap(&database_key(), &code, b"anna").unwrap();

        assert_eq!(unwrap(&wrapped, &code, b"anna").unwrap().expose(), &[0x42; 32]);
        assert_ne!(wrapped.ciphertext, vec![0x42; 32]);
    }

    #[test]
    fn another_code_or_account_does_not_open_the_key() {
        let code = new_code().unwrap();
        let wrapped = wrap(&database_key(), &code, b"anna").unwrap();

        assert_eq!(unwrap(&wrapped, &new_code().unwrap(), b"anna").unwrap_err(), RecoveryError::WrongCode);
        assert_eq!(unwrap(&wrapped, &code, b"jan").unwrap_err(), RecoveryError::WrongCode);
    }

    #[test]
    fn every_code_and_wrapping_is_new() {
        let code = new_code().unwrap();
        let first = wrap(&database_key(), &code, b"anna").unwrap();
        let second = wrap(&database_key(), &code, b"anna").unwrap();

        assert_ne!(code.expose(), new_code().unwrap().expose());
        assert_ne!(first.salt, second.salt);
        assert_ne!(first.nonce, second.nonce);
    }

    #[test]
    fn a_damaged_copy_is_refused() {
        let code = new_code().unwrap();
        let mut wrapped = wrap(&database_key(), &code, b"anna").unwrap();

        wrapped.tag.pop();
        assert_eq!(unwrap(&wrapped, &code, b"anna").unwrap_err(), RecoveryError::Damaged);

        let mut changed = wrap(&database_key(), &code, b"anna").unwrap();
        changed.ciphertext[0] ^= 1;
        assert_eq!(unwrap(&changed, &code, b"anna").unwrap_err(), RecoveryError::WrongCode);
    }
}
