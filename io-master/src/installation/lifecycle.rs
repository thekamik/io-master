//! The life of an installation: the first start, every later
//! start, and the removal of the program.
//!
//! An installation consists of three parts, all created at the
//! first start:
//!
//! ```text
//! the record          installation.json (later the system database)
//! the device secret   device-secret.sealed, sealed by the TPM
//! the marker          an entry in the TPM with the identifier
//! ```
//!
//! At every start [`open`] compares the record and the marker
//! (`docs/architecture.md`, section 3.2):
//!
//! ```text
//! record    marker in the TPM   state
//! missing   missing             first start: choose the role
//! present   same identifier     installed
//! missing   present             damaged: the record was removed
//! present   missing             damaged: other computer or TPM cleared
//! present   other identifier    damaged: the record belongs elsewhere
//! ```
//!
//! A damaged installation does not start. [`uninstall`] removes all
//! three parts, also of a damaged installation, so that a new first
//! start is possible.

use std::fmt;
use std::path::Path;

use crate::random::random::fill_random;
use crate::tpm::device_secret;
use crate::tpm::tpm::{Tpm, TpmError};

use super::installation::{self, Installation, InstallationError, InstallationId, Role, State};

/// Why an installation could not be opened, made or removed.
#[derive(Debug)]
pub enum LifecycleError {
    /// The parts of the installation do not belong together; holds
    /// the explanation. The program must not start.
    Damaged(String),
    /// The TPM already holds the marker of an installation: there
    /// is one installation per computer.
    AnotherInstallation,
    /// The record could not be read or written.
    Installation(InstallationError),
    /// The TPM failed.
    Tpm(TpmError),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::Damaged(reason) => write!(formatter, "the installation is damaged: {reason}"),
            LifecycleError::AnotherInstallation => {
                formatter.write_str("this computer already has an installation of IO Master")
            }
            LifecycleError::Installation(error) => write!(formatter, "{error}"),
            LifecycleError::Tpm(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<InstallationError> for LifecycleError {
    fn from(error: InstallationError) -> LifecycleError {
        match error {
            // A record that cannot be read is a damaged installation.
            InstallationError::Damaged { path, reason } => {
                LifecycleError::Damaged(format!("{}: {}", path.display(), reason))
            }
            other => LifecycleError::Installation(other),
        }
    }
}

impl From<TpmError> for LifecycleError {
    fn from(error: TpmError) -> LifecycleError {
        LifecycleError::Tpm(error)
    }
}

/// Tells the state of the installation in a data directory, and
/// checks that the device secret can be opened.
pub fn open(data_dir: &Path, tpm: &mut Tpm) -> Result<State, LifecycleError> {
    let marker = tpm.read_marker()?;
    let record = installation::load(data_dir)?;

    match (record, marker) {
        (State::FirstRun, None) => {
            // A sealed secret without record and marker is left from
            // a first start that was interrupted.
            if device_secret::path(data_dir).exists() {
                return Err(LifecycleError::Damaged(
                    "an earlier first start did not finish".to_owned(),
                ));
            }

            Ok(State::FirstRun)
        }
        (State::Installed(installation), Some(id)) if installation.id == id => {
            // The secret is opened once to prove that this TPM sealed
            // it. TODO (step 3 of docs/todo.md): keep it for the
            // databases instead of dropping it.
            device_secret::load(tpm, data_dir)?;

            Ok(State::Installed(installation))
        }
        (State::Installed(_), Some(_)) => Err(LifecycleError::Damaged(
            "the record belongs to another installation than the TPM".to_owned(),
        )),
        (State::FirstRun, Some(_)) => Err(LifecycleError::Damaged(
            "the TPM holds an installation whose record was removed".to_owned(),
        )),
        (State::Installed(_), None) => Err(LifecycleError::Damaged(
            "the TPM holds no installation: another computer, or the TPM was cleared".to_owned(),
        )),
    }
}

/// Makes the installation at the first start: the device secret,
/// the record with the role and the first user, and last the
/// marker in the TPM.
pub fn install(data_dir: &Path, tpm: &mut Tpm, role: Role, first_user: &str) -> Result<Installation, LifecycleError> {
    installation::check_user_name(first_user)?;

    if tpm.read_marker()?.is_some() {
        return Err(LifecycleError::AnotherInstallation);
    }

    let mut id: InstallationId = [0; installation::ID_LENGTH];

    fill_random(&mut id).map_err(|source| TpmError::io(data_dir, source))?;

    // The order matters only when the program is stopped in the
    // middle: whatever was made is then found by open() as damaged
    // and removed by uninstall().
    device_secret::create(tpm, data_dir)?;

    let installation = installation::create(data_dir, id, role, first_user)?;

    match tpm.write_marker(&id) {
        Ok(()) => Ok(installation),
        Err(TpmError::MarkerExists) => Err(LifecycleError::AnotherInstallation),
        Err(error) => Err(error.into()),
    }
}

/// Removes the installation: the marker from the TPM, the sealed
/// secret and the record. Works on a damaged installation too.
/// Other files of the data directory are left to the caller.
pub fn uninstall(data_dir: &Path, tpm: &mut Tpm) -> Result<(), LifecycleError> {
    tpm.remove_marker()?;
    device_secret::remove(data_dir)?;
    installation::remove(data_dir)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
