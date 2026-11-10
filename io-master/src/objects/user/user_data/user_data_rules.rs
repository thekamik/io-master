//! The account of the user: the first start (the installation, the user,
//! the two copies of the key of the database, the recovery code), the
//! login, the recovery of a forgotten password with the recovery code, the
//! change of the password and a new recovery code. The rules only: the data is written by the stores of
//! `objects/user/`, the keys are wrapped by `user_keys_rules/password.rs` and
//! `user_keys_rules/recovery.rs`. One Linux account holds one installation with one user
//! (`docs/architecture.md`, section 3).
//!
//! ```text
//! create_account(name, password, password repeated)
//!   system.v2.db made ── the installation (16 random bytes) ── the user
//!   the key of the database: 32 random bytes                              (RAM only)
//!   key ──password (password::wrap)──► WrappedKey ─┐
//!   key ──recovery code (recovery::wrap)──► RecoveryWrappedKey ─┤
//!   users/<id>.v2.db made with the key, its settings              │
//!   user_keys (system.v2.db) written last ◄────────────────────────┘   ── the key wiped
//!   ◄── the user, the open database, the recovery code (shown once, kept nowhere)
//!
//! The keys are the last thing a first start writes, so a user with keys is
//! a first start that ended. One cut off before that (the program killed, the
//! power lost) left a user nobody can log in as: it counts as not installed
//! ([`installed_user`]), and the next first start removes what it left
//! ([`clear_unfinished`]).
//!
//! log_in(name, password)
//!   the user by its name ── its WrappedKey ──password (password::unwrap)──► the key
//!   users/<id>.v2.db opened with it ── the key wiped
//!   ◄── the user, the open database
//!
//! recover(name, recovery code, new password, repeated)
//!   the user by its name ── its RecoveryWrappedKey ──code (recovery::unwrap)──► the key
//!   users/<id>.v2.db opened with it                (first: the key is the right one)
//!   renewal::renew_keys(key, new password) ──► both copies anew, a new code
//!   ◄── the user, the open database, the new code (shown once)
//!
//! change_password(user, current, new, repeated) / new_recovery_code(user, password)
//!   its WrappedKey ──current password (password::unwrap)──► the key
//!   renewal::renew_keys(key, new or same password) ──► both copies anew
//!   ◄── the new code (shown once); the old code, and an old password, open nothing
//! ```
//!
//! A copy of the key is bound to the id of its user (`binding`), which
//! never changes, so that a copy moved to another user does not open, and
//! a new name of the user keeps its key.

use std::fmt;
use std::fs::{self, DirBuilder};
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use crate::infrastructure::clock::now;
use crate::infrastructure::security::random::fill_random;
use crate::infrastructure::storage::encryption::database_key::DatabaseKey;
use crate::infrastructure::storage::encryption::generate_key::generate_database_key;
use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::system_database::{self, SystemDatabase};
use crate::infrastructure::storage::user_database::{self, UserDatabase};
use crate::objects::schema::{SYSTEM_TABLES, USER_TABLES};
use crate::objects::user::program_data::program_data_model::{Installation, Settings};
use crate::objects::user::program_data::program_data_store::{create_installation, create_settings};
use crate::objects::user::user_data::user_data_model::User;
use crate::objects::user::user_data::user_data_store::{create_user, read_users};
use crate::objects::user::user_keys::user_keys_model::UserKeys;
use crate::objects::user::user_keys::user_keys_rules::password::{self, KeyError};
use crate::objects::user::user_keys::user_keys_rules::recovery::{self, RecoveryError};
use crate::objects::user::user_keys::user_keys_rules::recovery_code::{RecoveryCode, RecoveryCodeError};
use crate::objects::user::user_keys::user_keys_rules::renewal::{RenewalError, renew_keys};
use crate::objects::user::user_keys::user_keys_store::{create_user_keys, read_user_keys};

/// What the first start and the recovery give: the user, its database
/// (open), and the new recovery code, which the window shows once.
pub struct OpenedAccount {
    pub user: User,
    pub database: UserDatabase,
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
    /// The name or the recovery code is wrong (the two look the same).
    WrongRecovery,
    /// The password of the user logged in is wrong (the change of the
    /// password, a new recovery code).
    WrongPassword,
    /// The typed text is no recovery code: its length, a character, or a
    /// typing error the check sum finds.
    Code(RecoveryCodeError),
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
            AccountError::WrongRecovery => formatter.write_str("the name or the recovery code is wrong"),
            AccountError::WrongPassword => formatter.write_str("the password is wrong"),
            AccountError::Code(error) => write!(formatter, "{error}"),
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
            RecoveryError::WrongCode => AccountError::WrongRecovery,
            RecoveryError::Damaged | RecoveryError::Failed(_) => AccountError::Key(error.to_string()),
        }
    }
}

impl From<RecoveryCodeError> for AccountError {
    fn from(error: RecoveryCodeError) -> AccountError {
        AccountError::Code(error)
    }
}

impl From<RenewalError> for AccountError {
    fn from(error: RenewalError) -> AccountError {
        match error {
            RenewalError::Password(error) => error.into(),
            RenewalError::Recovery(error) => error.into(),
            RenewalError::Db(error) => error.into(),
        }
    }
}

/// What a copy of the key is bound to: the id of its user.
///
/// Called by: [`keys_and_database`], [`log_in`], [`recover`],
/// [`open_with_password`], [`change_password`], [`new_recovery_code`].
fn binding(user: i64) -> Vec<u8> {
    format!("io-master user {user}").into_bytes()
}

/// Writes the installation, the user and the two copies of the key into
/// the new system database, and makes the database of the user.
///
/// Input: `system`, the new system database; `data_dir`, the data folder;
/// `name`, the name of the user; `password`, the password, checked.
/// Output: the id of the user (also when it fails after the user was
/// written, so that its database file can be removed) and the result: the
/// account opened, or the error (`(Some(1), Ok(opened))`).
///
/// Called by: [`create_account`].
/// Calls: `infrastructure/security/random.rs`: `fill_random`; `infrastructure/clock.rs`: `now`;
/// `objects/user/program_data/program_data_store.rs`: `create_installation`;
/// `objects/user/user_data/user_data_store.rs`: `create_user`; [`keys_and_database`].
fn install(
    system: &SystemDatabase,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> (Option<i64>, Result<OpenedAccount, AccountError>) {
    let mut id = [0u8; 16];

    if let Err(error) = fill_random(&mut id) {
        return (None, Err(AccountError::Key(error.to_string())));
    }

    let created = match now() {
        Ok(created) => created,
        Err(error) => return (None, Err(AccountError::Key(error.to_string()))),
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
/// the database of the user with its settings, and last the copies written.
///
/// Input: `system`, the new system database; `data_dir`, the data folder;
/// `user`, the user written; `password`, the password, checked.
/// Output: the account opened, with the recovery code to show once; an error
/// of the key, the copies or the databases.
///
/// Called by: [`install`].
/// Calls: `infrastructure/storage/encryption/generate_key.rs`: `generate_database_key`;
/// `objects/user/user_keys/user_keys_rules/recovery.rs`: `new_code`, `wrap`;
/// `objects/user/user_keys/user_keys_rules/password.rs`: `wrap`; [`binding`];
/// `infrastructure/storage/user_database.rs`: `create`; `objects/user/program_data/program_data_store.rs`:
/// `create_settings`; `objects/user/user_keys/user_keys_store.rs`: `create_user_keys`.
fn keys_and_database(
    system: &SystemDatabase,
    data_dir: &Path,
    user: User,
    password: &str,
) -> Result<OpenedAccount, AccountError> {
    let key = generate_database_key().map_err(|error| AccountError::Key(error.to_string()))?;
    let code = recovery::new_code()?;
    // `password::wrap` is the function of `user_keys_rules/password.rs`;
    // `password` beside it is the password the user gave.
    let keys = UserKeys {
        user: user.id,
        password: password::wrap(&key, password, &binding(user.id))?,
        recovery: recovery::wrap(&key, &code, &binding(user.id))?,
    };
    let database = user_database::create(data_dir, user.id, &key, &USER_TABLES)?;

    create_settings(&database, &Settings { active_integration: None })?;

    // The copies of the key last: once they are written the first start has
    // ended, and until then it can be made anew ([`clear_unfinished`]).
    create_user_keys(system, &keys)?;

    Ok(OpenedAccount { user, database, code })
}

/// Removes the file `path` a first start made; a missing file is no error.
///
/// Input: `path`, the system database or the database of a user.
/// Output: nothing; `AccountError::Key` naming the file when it cannot be
/// removed.
///
/// Called by: [`removed`], [`clear_unfinished`].
fn remove_leftover(path: &Path) -> Result<(), AccountError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AccountError::Key(format!("the file {} could not be removed: {error}", path.display()))),
    }
}

/// Removes a file of a first start that failed, so that the first start can
/// be tried again.
///
/// Input: `path`, a file the first start made; `failure`, why it failed.
/// Output: the failure; with what went wrong added when the file cannot be
/// removed.
///
/// Called by: [`create_account`].
/// Calls: [`remove_leftover`].
fn removed(path: &Path, failure: AccountError) -> AccountError {
    match remove_leftover(path) {
        Ok(()) => failure,
        Err(error) => AccountError::Key(format!("{failure}; {error}")),
    }
}

/// The user of the installation whose first start ended: the first user
/// with the copies of its key, which a first start writes last.
///
/// Input: `system`, the open system database.
/// Output: the user (`Some(User { name: "Kamil", .. })`); `None` without a
/// user, or with one that has no keys (a first start cut off); an error of
/// the database.
///
/// Called by: [`installed_user`], [`clear_unfinished`].
/// Calls: `objects/user/user_data/user_data_store.rs`: `read_users`;
/// `objects/user/user_keys/user_keys_store.rs`: `read_user_keys`.
fn finished_user(system: &SystemDatabase) -> Result<Option<User>, AccountError> {
    let Some(user) = read_users(system)?.into_iter().next() else {
        return Ok(None);
    };

    match read_user_keys(system, user.id) {
        Ok(_) => Ok(Some(user)),
        Err(DbError::NoRow { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Removes what a first start cut off half way left in the data folder
/// `data_dir`: the system database, and the database of its user, which no
/// key was written for, so that nobody could ever open it.
///
/// Input: `data_dir`, the data folder, its system database existing.
/// Output: nothing, the two files removed; `AccountError::AlreadyInstalled`
/// when the first start ended (a user with keys: nothing is removed), an
/// error of the database or of a file that cannot be removed.
///
/// Called by: [`create_account`].
/// Calls: [`open_installed`], [`finished_user`]; `objects/user/user_data/user_data_store.rs`: `read_users`;
/// `infrastructure/storage/user_database.rs`: `path`; `infrastructure/storage/system_database.rs`: `path`;
/// [`remove_leftover`].
fn clear_unfinished(data_dir: &Path) -> Result<(), AccountError> {
    let system = open_installed(data_dir)?;

    ensure!(finished_user(&system)?.is_none(), AccountError::AlreadyInstalled);

    // The users are bounded by their store; a first start writes one.
    for user in read_users(&system)? {
        remove_leftover(&user_database::path(data_dir, user.id))?;
    }

    drop(system);

    remove_leftover(&system_database::path(data_dir))
}

/// The first start: makes the data folder (private to the user of the
/// computer), the installation, its one user `name` with the password
/// `password` (given twice), and the database of the user. When it fails
/// half way, the files it made are removed; what a first start cut off left
/// behind is removed before it starts.
///
/// Input: `data_dir`, the data folder of the program; `name`, the name typed
/// ("Kamil"); `password`, `repeated`, the password given twice.
/// Output: the user, the open database and the recovery code, which the
/// window shows once; `AccountError::PasswordsDiffer`, a password refused,
/// `AccountError::AlreadyInstalled`, or an error of the folder, the key or
/// the databases.
///
/// Called by: `panels/views/program_start/installation.rs`: `installation_create`.
/// Calls: `objects/user/user_keys/user_keys_rules/password.rs`: `check_password`;
/// `infrastructure/storage/system_database.rs`: `create`, `path`; `infrastructure/storage/user_database.rs`:
/// `path`; [`clear_unfinished`], [`install`], [`removed`].
pub fn create_account(
    data_dir: &Path,
    name: &str,
    password: &str,
    repeated: &str,
) -> Result<OpenedAccount, AccountError> {
    ensure!(password == repeated, AccountError::PasswordsDiffer);
    // The module `user_keys_rules/password.rs` checks the password given.
    password::check_password(password)?;

    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(data_dir)
        .map_err(|error| AccountError::Key(format!("the data folder {}: {error}", data_dir.display())))?;

    let system = match system_database::create(data_dir, &SYSTEM_TABLES) {
        Ok(system) => system,
        // The file exists: an installation (refused), or what a first start
        // cut off left, removed so that the database is made anew.
        Err(DbError::Exists(_)) => {
            clear_unfinished(data_dir)?;
            system_database::create(data_dir, &SYSTEM_TABLES)?
        }
        Err(error) => return Err(error.into()),
    };
    let (user, result) = install(&system, data_dir, name, password);

    drop(system);

    match result {
        Ok(created) => Ok(created),
        Err(failure) => {
            let failure = match user {
                Some(user) => removed(&user_database::path(data_dir, user), failure),
                None => failure,
            };

            Err(removed(&system_database::path(data_dir), failure))
        }
    }
}

/// The user of the installation in the data folder `data_dir`.
///
/// Input: `data_dir`, the data folder of the program.
/// Output: the user (`Some(User { name: "Kamil", .. })`); `None` when the
/// program is not installed yet, or its first start was cut off half way, so
/// that the window shows the first start; an error of the database.
///
/// Called by: `panels/views/program_start/login.rs`: `login_get`.
/// Calls: `infrastructure/storage/system_database.rs`: `open`; [`finished_user`].
pub fn installed_user(data_dir: &Path) -> Result<Option<User>, AccountError> {
    let system = match system_database::open(data_dir, &SYSTEM_TABLES) {
        Ok(system) => system,
        Err(DbError::NotFound(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    finished_user(&system)
}

/// Opens the system database of an installed program.
///
/// Input: `data_dir`, the data folder of the program.
///
/// Output: the open system database; `AccountError::NotInstalled` before
/// the first start, or an error of the database.
///
/// Called by: [`clear_unfinished`], [`log_in`], [`recover`], [`open_with_password`].
/// Calls: `infrastructure/storage/system_database.rs`: `open`.
fn open_installed(data_dir: &Path) -> Result<SystemDatabase, AccountError> {
    match system_database::open(data_dir, &SYSTEM_TABLES) {
        Ok(system) => Ok(system),
        Err(DbError::NotFound(_)) => Err(AccountError::NotInstalled),
        Err(error) => Err(error.into()),
    }
}

/// The user named `name`, whatever its letter case and the spaces around
/// it ("kamil " finds "Kamil").
///
/// Input: `system`, the open system database; `name`, the name as typed.
///
/// Output: the user; `None` when no user has that name.
///
/// Called by: [`log_in`], [`recover`].
/// Calls: `objects/user/user_data/user_data_store.rs`: `read_users`.
fn find_user(system: &SystemDatabase, name: &str) -> Result<Option<User>, AccountError> {
    let wanted = name.trim().to_lowercase();

    ensure!(!wanted.is_empty(), AccountError::WrongCredentials);

    for user in read_users(system)? {
        if user.name.to_lowercase() == wanted {
            return Ok(Some(user));
        }
    }

    Ok(None)
}

/// The login: the user `name` (whatever its letter case) with its
/// password. The key is wiped when this function ends.
///
/// Input: `data_dir`, the data folder of the program; `name`, the name typed;
/// `password`, the password typed.
/// Output: the user and its database, opened; `AccountError::WrongCredentials`
/// for a wrong password and an unknown name alike, an error of the
/// databases.
///
/// Called by: `panels/views/program_start/login.rs`: `login_log_in`.
/// Calls: [`open_installed`], [`find_user`]; `objects/user/user_keys/user_keys_store.rs`: `read_user_keys`;
/// `objects/user/user_keys/user_keys_rules/password.rs`: `unwrap`; [`binding`];
/// `infrastructure/storage/user_database.rs`: `open`.
pub fn log_in(data_dir: &Path, name: &str, password: &str) -> Result<(User, UserDatabase), AccountError> {
    let system = open_installed(data_dir)?;
    let Some(user) = find_user(&system, name)? else {
        return Err(AccountError::WrongCredentials);
    };
    let keys = read_user_keys(&system, user.id)?;
    // `password::unwrap` (`user_keys_rules/password.rs`) opens the copy of the
    // key wrapped with the password (`keys.password`) with the password given.
    let key = password::unwrap(&keys.password, password, &binding(user.id))?;
    let database = user_database::open(data_dir, user.id, &key, &USER_TABLES)?;

    Ok((user, database))
}

/// The recovery of a forgotten password: the recovery code of the user
/// `name` opens the key of their database, which is then wrapped anew with
/// the new password and a new recovery code. Gives the user, their database
/// (open) and the new code, which the window shows once; the old code and
/// the forgotten password open nothing any more. A wrong code and an
/// unknown name fail the same way.
///
/// Input:
/// - `data_dir`: the data folder of the program.
/// - `name`: the name of the user, as typed ("kamil").
/// - `typed`: the recovery code as typed, the letter case, spaces and dashes
///   aside ("abcde-fghjk-...").
/// - `password`, `repeated`: the new password, given twice.
///
/// Output: the user, the open database and the new code; an error when the
/// passwords differ or are weak, the code is mistyped
/// (`AccountError::Code`), or the name or the code is wrong
/// (`AccountError::WrongRecovery`).
///
/// Called by: `panels/views/program_start/login.rs`: `login_recover`.
/// Calls: `objects/user/user_keys/user_keys_rules/password.rs`: `check_password`;
/// `objects/user/user_keys/user_keys_rules/recovery_code.rs`: `RecoveryCode::parse`; [`open_installed`],
/// [`find_user`]; `objects/user/user_keys/user_keys_store.rs`: `read_user_keys`;
/// `objects/user/user_keys/user_keys_rules/recovery.rs`: `unwrap`; `infrastructure/storage/user_database.rs`:
/// `open`; `objects/user/user_keys/user_keys_rules/renewal.rs`: `renew_keys`.
pub fn recover(
    data_dir: &Path,
    name: &str,
    typed: &str,
    password: &str,
    repeated: &str,
) -> Result<OpenedAccount, AccountError> {
    ensure!(password == repeated, AccountError::PasswordsDiffer);
    password::check_password(password)?;

    let code = RecoveryCode::parse(typed)?;
    let system = open_installed(data_dir)?;
    let Some(user) = find_user(&system, name)? else {
        return Err(AccountError::WrongRecovery);
    };
    let keys = read_user_keys(&system, user.id)?;
    let key = recovery::unwrap(&keys.recovery, &code, &binding(user.id))?;
    // The database is opened before the copies are written anew: a key that
    // does not open it (a damaged file) leaves the old copies as they were.
    let database = user_database::open(data_dir, user.id, &key, &USER_TABLES)?;
    let code = renew_keys(&system, user.id, &key, password, &binding(user.id))?;

    Ok(OpenedAccount { user, database, code })
}

/// Opens the key of the database of the user `user` with their password,
/// for a change made while they are logged in.
///
/// Input: `data_dir`, the data folder; `user`, the id of the user logged
/// in (1); `password`, their present password.
///
/// Output: the open system database and the key; `AccountError::WrongPassword`
/// when the password does not open it.
///
/// Called by: [`change_password`], [`new_recovery_code`].
/// Calls: [`open_installed`]; `objects/user/user_keys/user_keys_store.rs`: `read_user_keys`;
/// `objects/user/user_keys/user_keys_rules/password.rs`: `unwrap`.
fn open_with_password(
    data_dir: &Path,
    user: i64,
    password: &str,
) -> Result<(SystemDatabase, DatabaseKey), AccountError> {
    ensure!(user > 0, AccountError::Key("no user is named".to_owned()));

    let system = open_installed(data_dir)?;
    let keys = read_user_keys(&system, user)?;
    // A wrong password here is told as it is: the user is logged in, so
    // there is no name to keep secret.
    let key = match password::unwrap(&keys.password, password, &binding(user)) {
        Ok(key) => key,
        Err(KeyError::WrongPassword) => return Err(AccountError::WrongPassword),
        Err(error) => return Err(error.into()),
    };

    Ok((system, key))
}

/// Changes the password of the user `user`, logged in: their present
/// password opens the key of their database, which is wrapped anew with
/// the new password and a new recovery code. Gives the new code, which the
/// window shows once; the old password and the old code open nothing any
/// more.
///
/// Input: `data_dir`, the data folder; `user`, the id of the user logged
/// in (1); `current`, their present password; `password`, `repeated`, the
/// new one given twice.
///
/// Output: the new recovery code; an error when the new passwords differ
/// or are weak, or the present one is wrong (`AccountError::WrongPassword`).
///
/// Called by: `panels/views/account/credentials.rs`: `account_change_password`.
/// Calls: `objects/user/user_keys/user_keys_rules/password.rs`: `check_password`; [`open_with_password`];
/// `objects/user/user_keys/user_keys_rules/renewal.rs`: `renew_keys`.
pub fn change_password(
    data_dir: &Path,
    user: i64,
    current: &str,
    password: &str,
    repeated: &str,
) -> Result<RecoveryCode, AccountError> {
    ensure!(password == repeated, AccountError::PasswordsDiffer);
    password::check_password(password)?;

    let (system, key) = open_with_password(data_dir, user, current)?;

    Ok(renew_keys(&system, user, &key, password, &binding(user))?)
}

/// Makes a new recovery code for the user `user`, logged in, when the old
/// one is lost or may have been seen: their password opens the key, which
/// is wrapped anew with the same password and the new code. Gives the new
/// code, which the window shows once; the old one opens nothing any more.
///
/// Input: `data_dir`, the data folder; `user`, the id of the user logged
/// in (1); `password`, their password.
///
/// Output: the new recovery code; `AccountError::WrongPassword` when the
/// password is wrong.
///
/// Called by: `panels/views/account/credentials.rs`: `account_new_recovery_code`.
/// Calls: [`open_with_password`]; `objects/user/user_keys/user_keys_rules/renewal.rs`: `renew_keys`.
pub fn new_recovery_code(data_dir: &Path, user: i64, password: &str) -> Result<RecoveryCode, AccountError> {
    ensure!(!password.is_empty(), AccountError::WrongPassword);

    let (system, key) = open_with_password(data_dir, user, password)?;

    Ok(renew_keys(&system, user, &key, password, &binding(user))?)
}

#[cfg(test)]
mod tests {
    //! Tests of the account: the first start, the login, the checks, a
    //! first start that fails half way leaving nothing behind, one cut off
    //! half way made anew, the recovery
    //! with the code, the change of the password and a new code.

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

    #[test]
    fn a_first_start_cut_off_half_way_is_made_anew() {
        let directory = TempDir::new();

        // What a first start killed while Argon2id ran leaves: the system
        // database with its user and no keys, and here also the file of the
        // database of that user.
        let system = system_database::create(directory.path(), &SYSTEM_TABLES).unwrap();
        let id = create_user(&system, &User { id: 0, name: "Kamil".to_owned(), created: 1 }).unwrap();
        let key = generate_database_key().unwrap();

        drop(user_database::create(directory.path(), id, &key, &USER_TABLES).unwrap());
        drop(system);

        assert!(installed_user(directory.path()).unwrap().is_none());

        let created = create_account(directory.path(), "Anna", PASSWORD, PASSWORD).unwrap();
        let (logged, _database) = log_in(directory.path(), "anna", PASSWORD).unwrap();

        assert_eq!(logged, created.user);
    }

    const NEW_PASSWORD: &str = "another long password";

    #[test]
    fn the_recovery_code_sets_a_new_password_and_is_replaced() {
        let directory = TempDir::new();
        let created = create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();
        // Typed carelessly: small letters, spaces instead of dashes.
        let typed = created.code.to_text().to_lowercase().replace('-', " ");
        let recovered = recover(directory.path(), " kamil", &typed, NEW_PASSWORD, NEW_PASSWORD).unwrap();

        assert_eq!(recovered.user, created.user);
        assert_ne!(recovered.code.to_text(), created.code.to_text());
        assert!(log_in(directory.path(), "Kamil", NEW_PASSWORD).is_ok());
        assert!(matches!(log_in(directory.path(), "Kamil", PASSWORD), Err(AccountError::WrongCredentials)));
        assert!(matches!(
            recover(directory.path(), "Kamil", &typed, NEW_PASSWORD, NEW_PASSWORD),
            Err(AccountError::WrongRecovery)
        ));
        assert!(
            recover(directory.path(), "Kamil", &recovered.code.to_text(), PASSWORD, PASSWORD).is_ok(),
            "the new code works"
        );
    }

    #[test]
    fn a_wrong_name_a_mistyped_code_and_differing_passwords_are_refused() {
        let directory = TempDir::new();
        let created = create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();
        let code = created.code.to_text();

        assert!(matches!(
            recover(directory.path(), "Anna", &code, NEW_PASSWORD, NEW_PASSWORD),
            Err(AccountError::WrongRecovery)
        ));
        assert!(matches!(
            recover(directory.path(), "Kamil", "ABCDE", NEW_PASSWORD, NEW_PASSWORD),
            Err(AccountError::Code(RecoveryCodeError::Length))
        ));
        assert!(matches!(
            recover(directory.path(), "Kamil", &code, NEW_PASSWORD, "something else!"),
            Err(AccountError::PasswordsDiffer)
        ));
        assert!(log_in(directory.path(), "Kamil", PASSWORD).is_ok(), "nothing changed");
    }

    #[test]
    fn a_change_of_the_password_replaces_the_password_and_the_code() {
        let directory = TempDir::new();
        let created = create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();
        let id = created.user.id;

        assert!(matches!(
            change_password(directory.path(), id, "a wrong password", NEW_PASSWORD, NEW_PASSWORD),
            Err(AccountError::WrongPassword)
        ));

        let code = change_password(directory.path(), id, PASSWORD, NEW_PASSWORD, NEW_PASSWORD).unwrap();

        assert!(log_in(directory.path(), "Kamil", NEW_PASSWORD).is_ok());
        assert!(matches!(log_in(directory.path(), "Kamil", PASSWORD), Err(AccountError::WrongCredentials)));
        assert!(matches!(
            recover(directory.path(), "Kamil", &created.code.to_text(), PASSWORD, PASSWORD),
            Err(AccountError::WrongRecovery)
        ));
        assert!(recover(directory.path(), "Kamil", &code.to_text(), PASSWORD, PASSWORD).is_ok());
    }

    #[test]
    fn a_new_recovery_code_keeps_the_password() {
        let directory = TempDir::new();
        let created = create_account(directory.path(), "Kamil", PASSWORD, PASSWORD).unwrap();
        let id = created.user.id;

        assert!(matches!(
            new_recovery_code(directory.path(), id, "a wrong password"),
            Err(AccountError::WrongPassword)
        ));

        let code = new_recovery_code(directory.path(), id, PASSWORD).unwrap();

        assert!(log_in(directory.path(), "Kamil", PASSWORD).is_ok());
        assert!(matches!(
            recover(directory.path(), "Kamil", &created.code.to_text(), NEW_PASSWORD, NEW_PASSWORD),
            Err(AccountError::WrongRecovery)
        ));
        assert!(recover(directory.path(), "Kamil", &code.to_text(), NEW_PASSWORD, NEW_PASSWORD).is_ok());
    }
}
