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

use crate::db::system;
use std::fmt;
use std::path::Path;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::encryption::generate_key::generate_database_key;
use crate::db::encryption::keys::{self, KeyError};
use crate::db::error::DbError;
use crate::db::system::database::SystemDb;
use crate::db::user::database as user_db;
use crate::model::installation::{self, InvalidUserName};
use crate::security::tpm::device_secret::DeviceSecret;

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
    if !system::read::installation(system_db)?.role.can_add_users() {
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

    if system::read::user(system_db, name)?.is_some() {
        return Err(AccountError::NameTaken);
    }

    let key = generate_database_key().map_err(|error| AccountError::Key(KeyError::Failed(error.to_string())))?;
    let wrapped = keys::wrap(&key, password, device_secret, &binding(name))?;

    let id = system::create::add_user(system_db, name, &wrapped)?;

    // The database of the account. If it cannot be made, the row is
    // removed again, so that no account exists without its data.
    if let Err(error) = user_db::create(data_dir, id, &key) {
        let _ = system::delete::remove_user(system_db, id);

        return Err(error.into());
    }

    Ok(Account { id, name: name.to_owned() })
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
    let Some(user) = system::read::user(system_db, name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, password, device_secret, &binding(&user.name))?;

    // The database must open with the key; otherwise the account
    // is damaged, which is reported as such.
    user_db::open(data_dir, user.id, &key)?;

    Ok((Account { id: user.id, name: user.name }, key))
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

    let Some(user) = system::read::user(system_db, name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, old_password, device_secret, &binding(&user.name))?;
    let wrapped = keys::wrap(&key, new_password, device_secret, &binding(&user.name))?;

    system::update::replace_user_key(system_db, user.id, &wrapped)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `accounts.rs`, with the password as the only
    //! factor, as the program runs by default.

    use super::*;
    use crate::db::system;

    use crate::model::installation::{Installation, Role};
    use crate::test_support::TempDir;

    const PASSWORD: &str = "correct horse battery";

    /// A system database of the given role in a temporary directory.
    /// The third value is the secret of the device: none, as by
    /// default.
    fn setup(role: Role) -> (TempDir, SystemDb, Option<&'static DeviceSecret>) {
        let directory = TempDir::new();
        let installation = Installation { id: [1; 16], role };
        let db = SystemDb::create(directory.path(), &installation).unwrap();

        (directory, db, None)
    }

    #[test]
    fn a_created_account_logs_in_and_opens_its_database() {
        let (directory, db, device) = setup(Role::Workstation);

        let created = create_first(&db, device, directory.path(), "Anna", PASSWORD).unwrap();
        let (account, key) = log_in(&db, device, directory.path(), "anna", PASSWORD).unwrap();

        // The name as it was given, whatever was typed at login.
        assert_eq!(account, created);
        assert_eq!(account.name, "Anna");
        assert!(user_db::open(directory.path(), account.id, &key).is_ok());
    }

    #[test]
    fn a_wrong_password_and_an_unknown_name_look_the_same() {
        let (directory, db, device) = setup(Role::Workstation);

        create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();

        assert!(matches!(
            log_in(&db, device, directory.path(), "anna", "wrong horse battery"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(matches!(
            log_in(&db, device, directory.path(), "nobody", PASSWORD),
            Err(AccountError::WrongCredentials)
        ));
    }

    #[test]
    fn every_account_has_its_own_database_and_key() {
        let (directory, db, device) = setup(Role::Workstation);

        create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();
        add(&db, device, directory.path(), "jan", "another good password").unwrap();

        let (anna, anna_key) = log_in(&db, device, directory.path(), "anna", PASSWORD).unwrap();
        let (jan, jan_key) = log_in(&db, device, directory.path(), "jan", "another good password").unwrap();

        assert_ne!(anna_key.expose(), jan_key.expose());

        // The key of one account does not open the database of another.
        assert!(user_db::open(directory.path(), jan.id, &anna_key).is_err());
        assert!(user_db::open(directory.path(), anna.id, &jan_key).is_err());
    }

    #[test]
    fn a_master_keeps_its_one_user() {
        let (directory, db, device) = setup(Role::Master);

        create_first(&db, device, directory.path(), "admin", PASSWORD).unwrap();

        assert!(matches!(add(&db, device, directory.path(), "mallory", PASSWORD), Err(AccountError::NotAllowed)));
    }

    #[test]
    fn names_and_passwords_are_checked() {
        let (directory, db, device) = setup(Role::Workstation);

        create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();

        assert!(matches!(add(&db, device, directory.path(), "ANNA", PASSWORD), Err(AccountError::NameTaken)));
        assert!(matches!(add(&db, device, directory.path(), " ", PASSWORD), Err(AccountError::InvalidName(_))));
        assert!(matches!(add(&db, device, directory.path(), "jan", "short"), Err(AccountError::WeakPassword(_))));

        // Refused accounts leave nothing behind.
        assert_eq!(system::read::user_names(&db).unwrap(), ["anna"]);
    }

    #[test]
    fn a_changed_password_opens_the_same_database() {
        let (directory, db, device) = setup(Role::Workstation);

        create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();
        let (_, before) = log_in(&db, device, directory.path(), "anna", PASSWORD).unwrap();

        change_password(&db, device, "anna", PASSWORD, "a brand new password").unwrap();

        let (_, after) = log_in(&db, device, directory.path(), "anna", "a brand new password").unwrap();
        assert_eq!(before.expose(), after.expose());
        assert!(log_in(&db, device, directory.path(), "anna", PASSWORD).is_err());
    }

    #[test]
    fn a_password_is_changed_only_with_the_current_one() {
        let (directory, db, device) = setup(Role::Workstation);

        create_first(&db, device, directory.path(), "anna", PASSWORD).unwrap();

        assert!(matches!(
            change_password(&db, device, "anna", "not the password", "a brand new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(matches!(change_password(&db, device, "anna", PASSWORD, "short"), Err(AccountError::WeakPassword(_))));
        assert!(log_in(&db, device, directory.path(), "anna", PASSWORD).is_ok());
    }

    #[test]
    fn an_account_can_use_the_device_as_a_further_factor() {
        let (directory, db, _) = setup(Role::Workstation);
        let device = DeviceSecret::from_bytes([7; 32]);

        create_first(&db, Some(&device), directory.path(), "anna", PASSWORD).unwrap();

        assert!(log_in(&db, Some(&device), directory.path(), "anna", PASSWORD).is_ok());
        assert!(matches!(
            log_in(&db, None, directory.path(), "anna", PASSWORD),
            Err(AccountError::Key(KeyError::DeviceSecretMissing))
        ));
    }
}
