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

use crate::db::system;
use std::fmt;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use crate::db::encryption::keys;
use crate::db::error::DbError;
use crate::db::system::database::{self as system_db, SystemDb};
use crate::installation::accounts::{self, Account, AccountError};
use crate::security::random::fill_random;

use crate::model::installation::{self, Installation, InstallationId};
use crate::model::recovery_code::RecoveryCode;

/// An installed program, ready to work: its open system database.
#[derive(Debug)]
pub struct Running {
    pub system_db: SystemDb,
}

/// What a start finds.
#[derive(Debug)]
pub enum Opened {
    /// Nothing is installed: the user is made now.
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
    /// The first account could not be made, and the system database made
    /// for it could not be removed again: the next start would find an
    /// installation without a user. Holds both errors.
    AccountUnfinished(AccountError, DbError),
    /// A database or a file failed.
    Db(DbError),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::Damaged(reason) => write!(formatter, "the installation is damaged: {reason}"),
            LifecycleError::AlreadyInstalled => formatter.write_str("the program is already installed"),
            LifecycleError::Account(error) => write!(formatter, "{error}"),
            LifecycleError::AccountUnfinished(error, removal) => write!(
                formatter,
                "{error}; the started installation could not be removed ({removal}): remove the folder of IO Master by hand"
            ),
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
    // The record is read only to see that the database is whole; its
    // identifier is not needed while the program runs.
    system::read::installation(&system_db).map_err(|error| LifecycleError::Damaged(error.to_string()))?;

    Ok(Opened::Installed(Running { system_db }))
}

/// Makes the installation at the first start: the system database
/// and its account. Returns the running installation, the account and
/// its first recovery code, to be shown once (step 49 of docs/history.md).
///
/// Called by: `views/users/users.rs` (`setup_form_install`).
/// Calls: `SystemDb::create`, `accounts::create_first`.
pub fn install(
    data_dir: &Path,
    first_user: &str,
    password: &str,
) -> Result<(Running, Account, RecoveryCode), LifecycleError> {
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

    let installation = Installation { id };
    let system_db = SystemDb::create(data_dir, &installation)?;

    // If the first account cannot be made, the system database is
    // removed again: an installation without a user could never be
    // entered.
    let (account, code) = match accounts::create_first(&system_db, data_dir, first_user, password) {
        Ok(created) => created,
        Err(error) => {
            drop(system_db);

            return match SystemDb::remove(data_dir) {
                Ok(()) => Err(LifecycleError::Account(error)),
                Err(removal) => Err(LifecycleError::AccountUnfinished(error, removal)),
            };
        }
    };

    Ok((Running { system_db }, account, code))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `lifecycle.rs`: the first start, the next start
    //! and the removal.

    use super::*;
    use crate::db::system;

    use crate::test_support::TempDir;

    const PASSWORD: &str = "correct horse battery";

    /// Installs the program with the user "anna".
    fn install_program(directory: &TempDir) -> Running {
        install(directory.path(), "anna", PASSWORD).unwrap().0
    }

    #[test]
    fn no_system_database_is_the_first_start() {
        let directory = TempDir::new();

        assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));
    }

    #[test]
    fn an_installation_opens_at_the_next_start() {
        let directory = TempDir::new();
        install_program(&directory);

        let Opened::Installed(running) = open(directory.path()).unwrap() else {
            panic!("the installation was not found");
        };

        assert_eq!(system::read::user_names(&running.system_db).unwrap(), ["anna"]);

        // The account logs in with its password.
        assert!(accounts::log_in(&running.system_db, directory.path(), "anna", PASSWORD).is_ok());
    }

    #[test]
    fn the_installation_is_made_once() {
        let directory = TempDir::new();
        install_program(&directory);

        assert!(matches!(install(directory.path(), "mallory", PASSWORD), Err(LifecycleError::AlreadyInstalled)));

        let Opened::Installed(running) = open(directory.path()).unwrap() else {
            panic!("the installation was lost");
        };
        assert_eq!(system::read::user_names(&running.system_db).unwrap(), ["anna"]);
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
            assert!(matches!(install(directory.path(), name, password), Err(LifecycleError::Account(_))));
        }

        assert!(matches!(open(directory.path()).unwrap(), Opened::FirstRun));
    }
}
