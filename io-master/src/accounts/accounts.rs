//! The accounts of the evaluators: creating them, logging in, and
//! changing the password (`docs/architecture.md`, section 5.2).
//!
//! An account is a row of the system database and a database of its
//! own:
//!
//! ```text
//! system.db      users: name, encrypted key of the database
//! users/<id>.db  the data of the evaluator, encrypted with that key
//! ```
//!
//! Logging in means opening that key with the password, and with
//! the secret of the device when the optional TPM factor is used
//! (`db/encryption/keys.rs`). A wrong password, a wrong name, and an
//! account that does not exist all give the same answer, so the
//! answer does not tell which names exist.
//!
//! On a workstation every logged-in user may add further users; a
//! master keeps the one user who installed it.

use std::fmt;
use std::path::Path;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::error::DbError;
use crate::db::encryption::generate_key::generate_database_key;
use crate::db::encryption::keys::{self, KeyError};
use crate::db::system::database::SystemDb;
use crate::db::user::database as user_db;
use crate::installation::installation::{self, InvalidUserName};
use crate::tpm::device_secret::DeviceSecret;

/// A logged-in account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    /// The name as it was given when the account was created.
    pub name: String,
}

/// Why an account operation failed.
#[derive(Debug)]
pub enum AccountError {
    /// The name is not allowed; holds the reason.
    InvalidName(String),
    /// An account of that name exists (letter case aside).
    NameTaken,
    /// The password does not follow the rules; holds the reason.
    WeakPassword(String),
    /// The name or the password is wrong.
    WrongCredentials,
    /// The installation does not allow it: a master has one user.
    NotAllowed,
    /// The key of a database could not be made or opened.
    Key(KeyError),
    /// A database failed.
    Db(DbError),
}

impl fmt::Display for AccountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountError::InvalidName(reason) => write!(formatter, "invalid name: {reason}"),
            AccountError::NameTaken => formatter.write_str("an account of that name exists"),
            AccountError::WeakPassword(reason) => write!(formatter, "{reason}"),
            AccountError::WrongCredentials => formatter.write_str("wrong name or password"),
            AccountError::NotAllowed => formatter.write_str("a master has only one user"),
            AccountError::Key(error) => write!(formatter, "{error}"),
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
    fn from(error: KeyError) -> AccountError {
        match error {
            KeyError::WeakPassword(reason) => AccountError::WeakPassword(reason),
            KeyError::WrongPassword => AccountError::WrongCredentials,
            other => AccountError::Key(other),
        }
    }
}

impl From<InvalidUserName> for AccountError {
    fn from(error: InvalidUserName) -> AccountError {
        AccountError::InvalidName(error.0)
    }
}

/// What ties an encrypted key to its account: the name, in lower
/// case, as names are unique without regard to letter case.
fn binding(name: &str) -> Vec<u8> {
    format!("io-master account {}", name.to_lowercase()).into_bytes()
}

/// Creates the first account, at the first start.
pub fn create_first(
    system_db: &SystemDb,
    device_secret: Option<&DeviceSecret>,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<Account, AccountError> {
    create(system_db, device_secret, data_dir, name, password)
}

/// Adds an account on behalf of a logged-in user. Allowed only on
/// a workstation.
pub fn add(
    system_db: &SystemDb,
    device_secret: Option<&DeviceSecret>,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<Account, AccountError> {
    if !system_db.installation()?.role.can_add_users() {
        return Err(AccountError::NotAllowed);
    }

    create(system_db, device_secret, data_dir, name, password)
}

/// Creates an account: a new random key, the database it opens,
/// and the key stored encrypted with the password.
fn create(
    system_db: &SystemDb,
    device_secret: Option<&DeviceSecret>,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<Account, AccountError> {
    installation::check_user_name(name)?;
    keys::check_password(password)?;

    if system_db.user(name)?.is_some() {
        return Err(AccountError::NameTaken);
    }

    let key = generate_database_key().map_err(|error| AccountError::Key(KeyError::Failed(error.to_string())))?;
    let wrapped = keys::wrap(&key, password, device_secret, &binding(name))?;

    let id = system_db.add_user(name, &wrapped)?;

    // The database of the account. If it cannot be made, the row is
    // removed again, so that no account exists without its data.
    if let Err(error) = user_db::create(data_dir, id, &key) {
        let _ = system_db.remove_user(id);

        return Err(error.into());
    }

    Ok(Account {
        id,
        name: name.to_owned(),
    })
}

/// Logs in: opens the key of the account with the password, and its
/// database with that key. Returns the account and its key, which
/// the session keeps until logout.
pub fn log_in(
    system_db: &SystemDb,
    device_secret: Option<&DeviceSecret>,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<(Account, DatabaseKey), AccountError> {
    let Some(user) = system_db.user(name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, password, device_secret, &binding(&user.name))?;

    // The database must open with the key; otherwise the account
    // is damaged, which is reported as such.
    user_db::open(data_dir, user.id, &key)?;

    Ok((
        Account {
            id: user.id,
            name: user.name,
        },
        key,
    ))
}

/// Changes the password of an account. The old password is needed:
/// it opens the key, which is then encrypted with the new one. The
/// database itself is not touched.
pub fn change_password(
    system_db: &SystemDb,
    device_secret: Option<&DeviceSecret>,
    name: &str,
    old_password: &str,
    new_password: &str,
) -> Result<(), AccountError> {
    keys::check_password(new_password)?;

    let Some(user) = system_db.user(name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, old_password, device_secret, &binding(&user.name))?;
    let wrapped = keys::wrap(&key, new_password, device_secret, &binding(&user.name))?;

    system_db.replace_user_key(user.id, &wrapped)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "accounts_tests.rs"]
mod tests;
