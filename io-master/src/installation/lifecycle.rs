//! The life of an installation: the first start, every later
//! start, and the removal of the program.
//!
//! An installation consists of these parts, created at the first
//! start:
//!
//! ```text
//! the record          system.db, the system database
//! the first account   a row of system.db and users/<id>.db
//! ```
//!
//! **The only thing that decides whether this is the first start is
//! the system database**: without it the program offers the setup
//! page, with it the login (`docs/architecture.md`, section 3.2).
//!
//! The TPM is an optional, further factor (`security/tpm/`) and is not used
//! by default, so the program also runs on machines without a TPM,
//! such as many virtual machines. TODO (step 8 of docs/todo.md):
//! when the TPM factor is switched on, its secret is unsealed here
//! and its marker is compared with the record, as described in the
//! architecture.
//!
//! [`uninstall`] removes all parts, also of a damaged installation,
//! so that a new first start is possible.

use crate::db::system;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use crate::db::encryption::keys;
use crate::db::error::DbError;
use crate::db::system::database::{self as system_db, SystemDb};
use crate::db::user::database as user_db;
use crate::installation::accounts::{self, Account, AccountError};
use crate::security::random::fill_random;
use crate::security::tpm::device_secret::{self, DeviceSecret};
use crate::security::tpm::emulator::EMULATOR_DIRECTORY;

use crate::model::installation::{self, Installation, InstallationId, Role};

/// An installed program, ready to work: its record and its open
/// system database.
#[derive(Debug)]
pub struct Running {
    pub installation: Installation,
    pub system_db: SystemDb,
    /// The secret of the device, when the optional TPM factor is
    /// used; `None` by default.
    pub device_secret: Option<DeviceSecret>,
}

/// What a start finds.
#[derive(Debug)]
pub enum Opened {
    /// Nothing is installed: the role and the first user are chosen
    /// now.
    FirstRun,
    /// The installation is done and opened.
    Installed(Running),
}

/// Why an installation could not be opened, made or removed.
#[derive(Debug)]
pub enum LifecycleError {
    /// The record cannot be read; holds the explanation. The
    /// program must not start.
    Damaged(String),
    /// The installation exists already (made a moment earlier by
    /// another request).
    AlreadyInstalled,
    /// The first account could not be made.
    Account(AccountError),
    /// A database or a file failed.
    Db(DbError),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::Damaged(reason) => write!(formatter, "the installation is damaged: {reason}"),
            LifecycleError::AlreadyInstalled => formatter.write_str("the program is already installed"),
            LifecycleError::Account(error) => write!(formatter, "{error}"),
            LifecycleError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<DbError> for LifecycleError {
    fn from(error: DbError) -> LifecycleError {
        match error {
            DbError::Exists(_) => LifecycleError::AlreadyInstalled,
            // Not a system database of this program, for example one
            // left encrypted by an earlier development version.
            DbError::WrongKey(path) => {
                LifecycleError::Damaged(format!("{} is not a system database of IO Master", path.display()))
            }
            other => LifecycleError::Db(other),
        }
    }
}

/// Tells the state of the installation in a data directory, and
/// opens it when it is installed.
pub fn open(data_dir: &Path) -> Result<Opened, LifecycleError> {
    if !system_db::path(data_dir).exists() {
        return Ok(Opened::FirstRun);
    }

    let system_db = SystemDb::open(data_dir)?;
    let installation =
        system::read::installation(&system_db).map_err(|error| LifecycleError::Damaged(error.to_string()))?;

    Ok(Opened::Installed(Running { installation, system_db, device_secret: None }))
}

/// Makes the installation at the first start: the system database
/// with the role, and the first account. Returns the running
/// installation and the first account.
pub fn install(
    data_dir: &Path,
    role: Role,
    first_user: &str,
    password: &str,
) -> Result<(Running, Account), LifecycleError> {
    // Checked before anything is made, so that a mistake in the
    // form leaves nothing behind.
    installation::check_user_name(first_user).map_err(|error| LifecycleError::Account(error.into()))?;
    keys::check_password(password).map_err(|error| LifecycleError::Account(error.into()))?;

    // The data directory is private to the user.
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(data_dir)
        .map_err(|source| DbError::Io { path: data_dir.to_owned(), source })?;

    let mut id: InstallationId = [0; installation::ID_LENGTH];

    fill_random(&mut id).map_err(|source| DbError::Io { path: data_dir.to_owned(), source })?;

    let installation = Installation { id, role };
    let system_db = SystemDb::create(data_dir, &installation)?;

    // If the first account cannot be made, the system database is
    // removed again: an installation without a user could never be
    // entered.
    let account = match accounts::create_first(&system_db, None, data_dir, first_user, password) {
        Ok(account) => account,
        Err(error) => {
            drop(system_db);
            let _ = SystemDb::remove(data_dir);

            return Err(LifecycleError::Account(error));
        }
    };

    Ok((Running { installation, system_db, device_secret: None }, account))
}

/// Removes the installation: the system database and the databases
/// of the accounts, and what the optional TPM factor may have left
/// (its sealed secret and the state of the TPM emulator). Works on
/// a damaged installation too.
///
/// TODO (step 8 of docs/todo.md): with the TPM factor switched on,
/// also remove its marker from the TPM.
pub fn uninstall(data_dir: &Path) -> Result<(), LifecycleError> {
    SystemDb::remove(data_dir)?;
    user_db::remove_all(data_dir)?;

    device_secret::remove(data_dir).map_err(|error| {
        LifecycleError::Db(DbError::Io {
            path: device_secret::path(data_dir),
            source: io::Error::other(error.to_string()),
        })
    })?;

    let emulator_state = data_dir.join(EMULATOR_DIRECTORY);

    match fs::remove_dir_all(&emulator_state) {
        Err(source) if source.kind() != io::ErrorKind::NotFound => {
            Err(LifecycleError::Db(DbError::Io { path: emulator_state, source }))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `lifecycle.rs`: the first start, the next start
    //! and the removal, with passwords only (no TPM).

    use super::*;
    use crate::db::system;

    use crate::test_support::TempDir;

    const PASSWORD: &str = "correct horse battery";

    /// Installs a workstation with the user "anna".
    fn install_workstation(directory: &TempDir) -> Running {
        install(directory.path(), Role::Workstation, "anna", PASSWORD).unwrap().0
    }

    #[test]
    fn no_system_database_is_the_first_start() {
        let directory = TempDir::new();

        assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));
    }

    #[test]
    fn an_installation_opens_at_the_next_start() {
        let directory = TempDir::new();
        let installed = install_workstation(&directory);

        let Opened::Installed(running) = open(directory.path()).unwrap() else {
            panic!("the installation was not found");
        };

        assert_eq!(running.installation, installed.installation);
        assert_eq!(system::read::user_names(&running.system_db).unwrap(), ["anna"]);
        assert!(running.device_secret.is_none());

        // The first account logs in with its password alone.
        assert!(accounts::log_in(&running.system_db, None, directory.path(), "anna", PASSWORD).is_ok());
    }

    #[test]
    fn the_installation_is_made_once() {
        let directory = TempDir::new();
        install_workstation(&directory);

        assert!(matches!(
            install(directory.path(), Role::Master, "mallory", PASSWORD),
            Err(LifecycleError::AlreadyInstalled)
        ));

        let Opened::Installed(running) = open(directory.path()).unwrap() else {
            panic!("the installation was lost");
        };
        assert_eq!(running.installation.role, Role::Workstation);
    }

    #[test]
    fn a_file_that_is_not_a_system_database_is_damage() {
        let directory = TempDir::new();

        fs::write(system_db::path(directory.path()), b"not a database at all, just text").unwrap();

        assert!(matches!(open(directory.path()), Err(LifecycleError::Damaged(_))));
    }

    #[test]
    fn a_mistake_in_the_form_changes_nothing() {
        let directory = TempDir::new();

        for (name, password) in [(" ", PASSWORD), ("anna", "short")] {
            assert!(matches!(
                install(directory.path(), Role::Workstation, name, password),
                Err(LifecycleError::Account(_))
            ));
        }

        assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));
    }

    #[test]
    fn uninstall_leaves_nothing_and_allows_a_new_first_start() {
        let directory = TempDir::new();
        install_workstation(&directory);

        // What an earlier version with the TPM may have left.
        fs::write(device_secret::path(directory.path()), b"sealed").unwrap();
        fs::create_dir_all(directory.path().join(EMULATOR_DIRECTORY)).unwrap();

        uninstall(directory.path()).unwrap();

        assert!(!system_db::path(directory.path()).exists());
        assert!(!directory.path().join(user_db::DIRECTORY).exists());
        assert!(!device_secret::path(directory.path()).exists());
        assert!(!directory.path().join(EMULATOR_DIRECTORY).exists());
        assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));

        // Removing again is not an error.
        uninstall(directory.path()).unwrap();
    }
}
