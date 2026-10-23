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

// TODO (step 8 of docs/todo.md): the TPM is an optional factor that
//       is not switched on yet; until then only the tests use this
//       file.
#![allow(dead_code)]

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::security::random::fill_random;

use super::chip::{SealedEnvelope, Tpm, TpmError};

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
    pub fn expose(&self) -> &[u8; LENGTH] {
        &self.0
    }

    /// A secret with known bytes, for the tests of the modules that
    /// use it, so they need no TPM.
    #[cfg(test)]
    pub fn from_bytes(bytes: [u8; LENGTH]) -> DeviceSecret {
        DeviceSecret(Box::new(Zeroizing::new(bytes)))
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

#[cfg(test)]
mod tests {
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
}
