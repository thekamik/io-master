//! The keys of the databases of the evaluators
//! (`docs/architecture.md`, section 5.2).
//!
//! ```text
//! password ──Argon2id──┐
//!                      ├─HKDF─► unlocking key ──AES-256-GCM──► key of an evaluator's
//! [secret of the device]┘                                       database (random)
//! ```
//!
//! - The database of an **evaluator** has a random key. That key is
//!   stored only encrypted ([`WrappedKey`]) with an unlocking key.
//!   The password itself is never stored: a wrong password simply
//!   fails to decrypt (GCM detects it).
//! - **Factors.** The password is the only factor that is always
//!   needed. Every further factor is added to the unlocking key and
//!   makes it stronger: today the optional secret of the device,
//!   sealed by the TPM (`security/tpm/`). A [`WrappedKey`] records which
//!   factors it needs.
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

use crate::security::random::fill_random;
use crate::security::tpm::device_secret::DeviceSecret;

use super::database_key::{DATABASE_KEY_LENGTH, DatabaseKey};

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
    /// The password (or the secret of the device) does not open
    /// the key.
    WrongPassword,
    /// The key needs the secret of the device, which is not there:
    /// the TPM factor is not available.
    DeviceSecretMissing,
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
            KeyError::DeviceSecretMissing => {
                formatter.write_str("the key needs the secret of the device (TPM), which is not available")
            }
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

/// The key of a database, encrypted with an unlocking key.
/// Everything needed to open it again, except the password and the
/// secret of the device; none of it is secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedKey {
    /// The random salt of Argon2id.
    pub salt: Vec<u8>,
    /// The parameters of Argon2id used, so that they can be raised
    /// later without locking out existing accounts.
    pub memory_kib: u32,
    pub iterations: u32,
    /// Whether the secret of the device (the TPM factor) is part of
    /// the unlocking key.
    pub with_device_secret: bool,
    /// The nonce of AES-GCM: random, used once.
    pub nonce: Vec<u8>,
    /// The encrypted key and its authentication tag.
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
}

/// Derives the unlocking key from a password and, when given, the
/// secret of the device.
fn unlocking_key(
    password: &str,
    salt: &[u8],
    memory_kib: u32,
    iterations: u32,
    device_secret: Option<&DeviceSecret>,
) -> Result<Zeroizing<[u8; 32]>, KeyError> {
    let params =
        Params::new(memory_kib, iterations, 1, Some(32)).map_err(|error| KeyError::Failed(error.to_string()))?;

    // Argon2id of the password, then together with the further
    // factors through HKDF.
    let mut stretched = Zeroizing::new([0u8; 32]);

    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut *stretched)
        .map_err(|error| KeyError::Failed(error.to_string()))?;

    let mut material = Zeroizing::new(Vec::with_capacity(64));
    material.extend_from_slice(&*stretched);

    if let Some(device_secret) = device_secret {
        material.extend_from_slice(device_secret.expose());
    }

    let mut key = Zeroizing::new([0u8; 32]);

    Hkdf::<Sha256>::new(Some(salt), &material)
        .expand(UNLOCKING_KEY_PURPOSE, &mut *key)
        .map_err(|error| KeyError::Failed(error.to_string()))?;

    Ok(key)
}

/// Encrypts the key of a database with a password and, when given,
/// the secret of the device as a further factor.
///
/// `binding` ties the result to one account (for example its
/// name): an encrypted key copied to another account does not open
/// there.
pub fn wrap(
    key: &DatabaseKey,
    password: &str,
    device_secret: Option<&DeviceSecret>,
    binding: &[u8],
) -> Result<WrappedKey, KeyError> {
    check_password(password)?;

    let mut salt = vec![0u8; SALT_LENGTH];
    let mut nonce = [0u8; NONCE_LENGTH];

    fill_random(&mut salt).map_err(|error| KeyError::Failed(error.to_string()))?;
    fill_random(&mut nonce).map_err(|error| KeyError::Failed(error.to_string()))?;

    let unlocking = unlocking_key(password, &salt, ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, device_secret)?;
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
        with_device_secret: device_secret.is_some(),
        nonce: nonce.to_vec(),
        ciphertext: buffer,
        tag: tag.to_vec(),
    })
}

/// Opens the key of a database with the password and the factors it
/// was wrapped with. Fails with [`KeyError::WrongPassword`] when
/// they do not fit, or when `binding` is not the one used by
/// [`wrap`], and with [`KeyError::DeviceSecretMissing`] when the key
/// needs the secret of the device and none is given.
pub fn unwrap(
    wrapped: &WrappedKey,
    password: &str,
    device_secret: Option<&DeviceSecret>,
    binding: &[u8],
) -> Result<DatabaseKey, KeyError> {
    // Exactly the factors the key was made with.
    let device_secret = match (wrapped.with_device_secret, device_secret) {
        (true, None) => return Err(KeyError::DeviceSecretMissing),
        (true, Some(secret)) => Some(secret),
        (false, _) => None,
    };

    let nonce: [u8; NONCE_LENGTH] = wrapped.nonce.as_slice().try_into().map_err(|_| KeyError::Damaged)?;
    let tag: [u8; TAG_LENGTH] = wrapped.tag.as_slice().try_into().map_err(|_| KeyError::Damaged)?;

    if wrapped.ciphertext.len() != DATABASE_KEY_LENGTH || wrapped.salt.len() != SALT_LENGTH {
        return Err(KeyError::Damaged);
    }

    let unlocking = unlocking_key(password, &wrapped.salt, wrapped.memory_kib, wrapped.iterations, device_secret)?;
    let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*unlocking));

    // Decrypted straight into the memory of the new key.
    DatabaseKey::fill_with(|bytes| {
        bytes.copy_from_slice(&wrapped.ciphertext);

        cipher
            .decrypt_inout_detached(&Nonce::from(nonce), binding, bytes.as_mut_slice().into(), &tag.into())
            .map_err(|_| KeyError::WrongPassword)
    })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
