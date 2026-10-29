//! The account of the user: the first start (the installation, the user,
//! the two copies of the key of the database, the recovery code) and the
//! login. The logic only: the data is written by `db/`, the keys are
//! wrapped by `keys.rs` and `recovery.rs`. One Linux account holds one
//! installation with one user (`docs/architecture.md`, section 3).
//!
//! ```text
//! create_account(name, password, password repeated)
//!   system.v2.db made ── the installation (16 random bytes) ── the user
//!   the key of the database: 32 random bytes                              (RAM only)
//!   key ──password (keys::wrap)──► WrappedKey ─┐
//!   key ──recovery code (recovery::wrap)──► RecoveryWrappedKey ─┴──► user_keys (system.v2.db)
//!   users/<id>.v2.db made with the key, its settings ── the key wiped
//!   ◄── the user, the open database, the recovery code (shown once, kept nowhere)
//!
//! log_in(name, password)
//!   the user by its name ── its WrappedKey ──password (keys::unwrap)──► the key
//!   users/<id>.v2.db opened with it ── the key wiped
//!   ◄── the user, the open database
//! ```
//!
//! A copy of the key is bound to the id of its user (`binding`), which
//! never changes, so that a copy moved to another user does not open, and
//! a new name of the user keeps its key.

use std::fmt;
use std::fs::{self, DirBuilder};
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::database::{self, Database};
use crate::db::error::DbError;
use crate::db::system_database::{self, SystemDatabase};
use crate::db::user::program_data::{create_installation, create_settings};
use crate::db::user::user_data::{create_user, read_users};
use crate::db::user::user_keys::{create_user_keys, read_user_keys};
use crate::model::user::program_data::{Installation, Settings};
use crate::model::user::user_data::User;
use crate::model::user::user_keys::UserKeys;
use crate::panels::logic::program_start::generate_key::generate_database_key;
use crate::panels::logic::program_start::keys::{self, KeyError};
use crate::panels::logic::program_start::recovery::{self, RecoveryError};
use crate::panels::logic::program_start::recovery_code::RecoveryCode;
use crate::security::random::fill_random;

/// What the first start gives: the user, its database (open), and the
/// recovery code, which the window shows once.
pub struct NewAccount {
    pub user: User,
    pub database: Database,
    pub code: RecoveryCode,
}

/// Why an account could not be made or opened. The window tells the kinds
/// apart; none of them holds a password or a key.
#[derive(Debug)]
pub enum AccountError {
    /// The password and its repetition differ.
    PasswordsDiffer,
    /// The password does not follow the rules; holds the reason.
    WeakPassword(String),
    /// The name or the password is wrong (the two look the same, so that
    /// nobody learns which names exist).
    WrongCredentials,
    /// The program is installed already: its one user exists.
    AlreadyInstalled,
    /// The program is not installed yet.
    NotInstalled,
    /// A key could not be made or opened (not a wrong password).
    Key(String),
    /// A database failed.
    Db(DbError),
}

impl fmt::Display for AccountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountError::PasswordsDiffer => formatter.write_str("the two passwords differ"),
            AccountError::WeakPassword(reason) => formatter.write_str(reason),
            AccountError::WrongCredentials => formatter.write_str("the name or the password is wrong"),
            AccountError::AlreadyInstalled => formatter.write_str("the program is installed already"),
            AccountError::NotInstalled => formatter.write_str("the program is not installed yet"),
            AccountError::Key(reason) => write!(formatter, "the key of the database: {reason}"),
            AccountError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AccountError {}

impl From<DbError> for AccountError {
    fn from(error: DbError) -> AccountError {
        AccountError::Db(error)
    }
}

impl From<KeyError> for AccountError {
    /// A wrong password is a wrong credential, like an unknown name.
    fn from(error: KeyError) -> AccountError {
        match error {
            KeyError::WeakPassword(reason) => AccountError::WeakPassword(reason),
            KeyError::WrongPassword => AccountError::WrongCredentials,
            KeyError::Damaged | KeyError::Failed(_) => AccountError::Key(error.to_string()),
        }
    }
}

impl From<RecoveryError> for AccountError {
    /// A wrong recovery code is a wrong credential.
    fn from(error: RecoveryError) -> AccountError {
        match error {
            RecoveryError::WrongCode => AccountError::WrongCredentials,
            RecoveryError::Damaged | RecoveryError::Failed(_) => AccountError::Key(error.to_string()),
        }
    }
}

/// What a copy of the key is bound to: the id of its user.
///
/// Called by: [`install`], [`log_in`].
fn binding(user: i64) -> Vec<u8> {
    format!("io-master user {user}").into_bytes()
}

/// The time now, in seconds since 1970 (UTC).
///
/// Called by: [`install`].
fn now() -> Result<i64, AccountError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AccountError::Key(format!("the clock: {error}")))?
        .as_secs();

    i64::try_from(seconds).map_err(|error| AccountError::Key(format!("the clock: {error}")))
}

/// Writes the installation, the user and the two copies of the key into
/// the new system database, and makes the database of the user. Gives the
/// id of the user (also when it fails after the user was written, so that
/// its database file can be removed), and the result.
///
/// Called by: [`create_account`].
/// Calls: `create_installation`, `create_user`, `generate_database_key`,
/// `keys::wrap`, `recovery::new_code`, `recovery::wrap`, `create_user_keys`,
/// `database::create`, `create_settings`.
fn install(
    system: &SystemDatabase,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> (Option<i64>, Result<NewAccount, AccountError>) {
    let mut id = [0u8; 16];

    if let Err(error) = fill_random(&mut id) {
        return (None, Err(AccountError::Key(error.to_string())));
    }

    let created = match now() {
        Ok(created) => created,
        Err(error) => return (None, Err(error)),
    };

    if let Err(error) = create_installation(system, &Installation { id, created }) {
        return (None, Err(error.into()));
    }

    let user = User { id: 0, name: name.to_owned(), created };
    let user_id = match create_user(system, &user) {
        Ok(user_id) => user_id,
        Err(error) => return (None, Err(error.into())),
    };

    (Some(user_id), keys_and_database(system, data_dir, User { id: user_id, ..user }, password))
}

/// The second half of [`install`]: the key of the database, its two copies,
/// the database of the user with its settings.
///
/// Called by: [`install`].
fn keys_and_database(
    system: &SystemDatabase,
    data_dir: &Path,
    user: User,
    password: &str,
) -> Result<NewAccount, AccountError> {
    let key = generate_database_key().map_err(|error| AccountError::Key(error.to_string()))?;
    let code = recovery::new_code()?;
    let keys = UserKeys {
        user: user.id,
        password: keys::wrap(&key, password, &binding(user.id))?,
        recovery: recovery::wrap(&key, &code, &binding(user.id))?,
    };

    create_user_keys(system, &keys)?;

    let database = database::create(data_dir, user.id, &key)?;

    create_settings(&database, &Settings { active_integration: None })?;

    Ok(NewAccount { user, database, code })
}

/// Removes a file of a first start that failed, so that the first start can
/// be tried again; a missing file is no error. A file that cannot be removed
/// is told with the failure.
///
/// Called by: [`create_account`].
fn removed(path: &Path, failure: AccountError) -> AccountError {
    match fs::remove_file(path) {
        Ok(()) => failure,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => failure,
        Err(error) => {
            AccountError::Key(format!("{failure}; the file {} could not be removed: {error}", path.display()))
        }
    }
}

/// The first start: makes the data folder (private to the user of the
/// computer), the installation, its one user `name` with the password
/// `password` (given twice), and the database of the user. Gives the user,
/// the open database and the recovery code, which the window shows once.
/// When it fails half way, the files it made are removed.
///
/// Called by: `panels/views/program_start/installation.rs`.
/// Calls: `keys::check_password`, `system_database::create`, [`install`],
/// [`removed`].
pub fn create_account(data_dir: &Path, name: &str, password: &str, repeated: &str) -> Result<NewAccount, AccountError> {
    ensure!(password == repeated, AccountError::PasswordsDiffer);
    keys::check_password(password)?;

    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(data_dir)
        .map_err(|error| AccountError::Key(format!("the data folder {}: {error}", data_dir.display())))?;

    let system = match system_database::create(data_dir) {
        Ok(system) => system,
        Err(DbError::Exists(_)) => return Err(AccountError::AlreadyInstalled),
        Err(error) => return Err(error.into()),
    };
    let (user, result) = install(&system, data_dir, name, password);

    drop(system);

    match result {
        Ok(created) => Ok(created),
        Err(failure) => {
            let failure = match user {
                Some(user) => removed(&database::path(data_dir, user), failure),
                None => failure,
            };

            Err(removed(&system_database::path(data_dir), failure))
        }
    }
}

/// The user of the installation in the data folder `data_dir`; `None` when
/// the program is not installed yet, so that the window shows the first
/// start.
///
/// Called by: `panels/views/program_start/login.rs`.
/// Calls: `system_database::open`, `read_users`.
pub fn installed_user(data_dir: &Path) -> Result<Option<User>, AccountError> {
    let system = match system_database::open(data_dir) {
        Ok(system) => system,
        Err(DbError::NotFound(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    Ok(read_users(&system)?.into_iter().next())
}

/// The login: the user `name` (whatever its letter case) with its
/// password. Gives the user and its database, opened; the key is wiped
/// when this function ends. A wrong password and an unknown name fail the
/// same way.
///
/// Called by: `panels/views/program_start/login.rs`.
/// Calls: `system_database::open`, `read_users`, `read_user_keys`,
/// `keys::unwrap`, `database::open`.
pub fn log_in(data_dir: &Path, name: &str, password: &str) -> Result<(User, Database), AccountError> {
    let system = match system_database::open(data_dir) {
        Ok(system) => system,
        Err(DbError::NotFound(_)) => return Err(AccountError::NotInstalled),
        Err(error) => return Err(error.into()),
    };
    let wanted = name.trim().to_lowercase();
    let mut found = None;

    for user in read_users(&system)? {
        if user.name.to_lowercase() == wanted {
            found = Some(user);
        }
    }

    let Some(user) = found else {
        return Err(AccountError::WrongCredentials);
    };
    let keys = read_user_keys(&system, user.id)?;
    let key = keys::unwrap(&keys.password, password, &binding(user.id))?;
    let database = database::open(data_dir, user.id, &key)?;

    Ok((user, database))
}

#[cfg(test)]
mod tests {
    //! Tests of the account: the first start, the login, the checks, and a
    //! first start that fails half way leaving nothing behind.

    use super::*;

    use crate::test_support::TempDir;

    const PASSWORD: &str = "a long enough password";

    #[test]
    fn the_first_start_makes_the_user_who_logs_in_whatever_the_letter_case() {
        let directory = TempDir::new();
        let created = create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();

        assert_eq!(created.code.to_text().len(), 41);
        assert_eq!(installed_user(directory.path()).unwrap(), Some(created.user.clone()));

        let (logged, _database) = log_in(directory.path(), "kamil", PASSWORD).unwrap();

        assert_eq!(logged, created.user);
        assert!(matches!(
            create_account(directory.path(), "Anna", PASSWORD, PASSWORD),
            Err(AccountError::AlreadyInstalled)
        ));
    }

    #[test]
    fn a_wrong_password_and_an_unknown_name_fail_the_same_way() {
        let directory = TempDir::new();

        assert!(matches!(log_in(directory.path(), "Kamil", PASSWORD), Err(AccountError::NotInstalled)));

        create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();

        assert!(matches!(log_in(directory.path(), "Kamil", "a wrong password!"), Err(AccountError::WrongCredentials)));
        assert!(matches!(log_in(directory.path(), "Anna", PASSWORD), Err(AccountError::WrongCredentials)));
    }

    #[test]
    fn the_passwords_are_checked_before_anything_is_made() {
        let directory = TempDir::new();

        assert!(matches!(
            create_account(directory.path(), "Kamil", PASSWORD, "another one!!"),
            Err(AccountError::PasswordsDiffer)
        ));
        assert!(matches!(
            create_account(directory.path(), "Kamil", "short", "short"),
            Err(AccountError::WeakPassword(_))
        ));
        assert!(installed_user(directory.path()).unwrap().is_none());
    }

    #[test]
    fn a_first_start_that_fails_leaves_nothing_and_can_be_tried_again() {
        let directory = TempDir::new();

        assert!(matches!(create_account(directory.path(), "a/b", PASSWORD, PASSWORD), Err(AccountError::Db(_))));
        assert!(!system_database::path(directory.path()).exists());
        assert!(installed_user(directory.path()).unwrap().is_none());

        create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();
    }
}
