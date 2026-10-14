//! The secret of the device (`docs/architecture.md`, section 5.2).
//!
//! 32 random bytes that exist only on this computer: they are kept
//! sealed by the TPM, in a file of the data directory:
//!
//! ```text
//! <data directory>/device-secret.sealed
//! ```
//!
//! The file is useless without the TPM that sealed it, so it is
//! not a secret itself; it is still readable only by the user.
//! Together with the password of an evaluator the secret unlocks
//! the database (step 3 of `docs/todo.md`).

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::random::random::fill_random;

use super::tpm::{SealedEnvelope, Tpm, TpmError};

/// Name of the file with the sealed secret.
pub const FILE_NAME: &str = "device-secret.sealed";

/// Length of the secret in bytes: 256 bits.
pub const LENGTH: usize = 32;

/// The secret of the device in memory.
///
/// Wiped when dropped, kept on the heap so that moving it does not
/// leave copies, never shown by `{:?}`, and given out only by
/// [`DeviceSecret::expose`].
pub struct DeviceSecret(Box<Zeroizing<[u8; LENGTH]>>);

impl DeviceSecret {
    /// The bytes of the secret, for deriving the keys of the
    /// databases.
    // TODO: used by the databases (step 3 of docs/todo.md); until
    //       then only by the tests.
    #[allow(dead_code)]
    pub fn expose(&self) -> &[u8; LENGTH] {
        &self.0
    }
}

impl fmt::Debug for DeviceSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeviceSecret(<hidden>)")
    }
}

/// The path of the sealed secret in a data directory.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Creates a new secret of the device, seals it with the TPM and
/// writes the envelope. Fails when an envelope exists already: a
/// secret is never replaced, since that would lose the data it
/// protects.
pub fn create(tpm: &mut Tpm, data_dir: &Path) -> Result<DeviceSecret, TpmError> {
    let mut secret = DeviceSecret(Box::new(Zeroizing::new([0; LENGTH])));

    fill_random(&mut secret.0[..]).map_err(|source| TpmError::io(data_dir, source))?;

    let envelope = tpm.seal(&secret.0[..])?;
    let path = path(data_dir);

    // create_new: never overwrite an existing secret.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|source| TpmError::io(&path, source))?;

    file.write_all(&envelope.to_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|source| TpmError::io(&path, source))?;

    Ok(secret)
}

/// Opens the sealed secret of the device. Fails when there is
/// none, when it is damaged, or when another TPM sealed it.
pub fn load(tpm: &mut Tpm, data_dir: &Path) -> Result<DeviceSecret, TpmError> {
    let path = path(data_dir);
    let bytes = fs::read(&path).map_err(|source| TpmError::io(&path, source))?;

    let opened = tpm.unseal(&SealedEnvelope::from_bytes(&bytes)?)?;

    let mut secret = DeviceSecret(Box::new(Zeroizing::new([0; LENGTH])));

    if opened.len() != LENGTH {
        return Err(TpmError::DamagedEnvelope);
    }

    secret.0.copy_from_slice(&opened);

    Ok(secret)
}

/// Removes the sealed secret; used when the program is
/// uninstalled. A missing file is not an error.
pub fn remove(data_dir: &Path) -> Result<(), TpmError> {
    let path = path(data_dir);

    match fs::remove_file(&path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(TpmError::io(&path, error)),
        _ => Ok(()),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "device_secret_tests.rs"]
mod tests;
