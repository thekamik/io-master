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
//! Logging in means opening that key with the password
//! (`db/encryption/keys.rs`). A wrong password, a wrong name, and an
//! account that does not exist all give the same answer, so the
//! answer does not tell which names exist.
//!
//! An installation has one account, made at the first start (step 48
//! of docs/history.md); no further account can be added.

use crate::db::system;
use std::fmt;
use std::path::Path;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::encryption::generate_key::generate_database_key;
use crate::db::encryption::keys::{self, KeyError, WrappedKey};
use crate::db::encryption::recovery::{self, RecoveryError, RecoveryWrappedKey};
use crate::db::error::DbError;
use crate::db::system::database::SystemDb;
use crate::db::user::database as user_db;
use crate::model_old::installation::{self, InvalidUserName};
use crate::model_old::recovery_code::RecoveryCode;

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
    /// The key of a database could not be made or opened.
    Key(KeyError),
    /// The copy of the key for the recovery code could not be made or
    /// opened (a wrong code is `WrongCredentials`).
    Recovery(RecoveryError),
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
            AccountError::Key(error) => write!(formatter, "{error}"),
            AccountError::Recovery(error) => write!(formatter, "{error}"),
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

impl From<RecoveryError> for AccountError {
    fn from(error: RecoveryError) -> AccountError {
        match error {
            // The same answer as a wrong name or password.
            RecoveryError::WrongCode => AccountError::WrongCredentials,
            other => AccountError::Recovery(other),
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

/// The key of a database wrapped twice for the account `name`: with the
/// password, and with a new recovery code, which is given back to be shown
/// once (step 49 of docs/history.md).
///
/// Called by: [`create`], [`change_password`], [`recover`].
/// Calls: `keys::wrap`, `recovery::new_code`, `recovery::wrap`.
fn wrap_twice(
    key: &DatabaseKey,
    password: &str,
    name: &str,
) -> Result<(WrappedKey, RecoveryWrappedKey, RecoveryCode), AccountError> {
    let wrapped = keys::wrap(key, password, &binding(name))?;
    let code = recovery::new_code()?;
    let recovery = recovery::wrap(key, &code, &binding(name))?;

    Ok((wrapped, recovery, code))
}

/// Creates the account at the first start; gives back its account and its
/// first recovery code, made automatically (user, 2026-10-24).
///
/// Called by: `installation/lifecycle.rs` (`install`).
/// Calls: [`create`].
pub fn create_first(
    system_db: &SystemDb,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<(Account, RecoveryCode), AccountError> {
    create(system_db, data_dir, name, password)
}

/// Creates an account: a new random key, the database it opens, and the
/// key stored wrapped with the password and with a new recovery code.
///
/// Called by: [`create_first`].
/// Calls: `check_user_name`, `keys::check_password`, `generate_database_key`,
/// [`wrap_twice`], `system::create::add_user`, `user_db::create`.
fn create(
    system_db: &SystemDb,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<(Account, RecoveryCode), AccountError> {
    installation::check_user_name(name)?;
    keys::check_password(password)?;

    if system::read::user(system_db, name)?.is_some() {
        return Err(AccountError::NameTaken);
    }

    let key = generate_database_key().map_err(|error| AccountError::Key(KeyError::Failed(error.to_string())))?;
    let (wrapped, recovery, code) = wrap_twice(&key, password, name)?;

    let id = system::create::add_user(system_db, name, &wrapped, &recovery)?;

    // The database of the account. If it cannot be made, the row is
    // removed again, so that no account exists without its data.
    if let Err(error) = user_db::create(data_dir, id, &key) {
        let _ = system::delete::remove_user(system_db, id);

        return Err(error.into());
    }

    Ok((Account { id, name: name.to_owned() }, code))
}

/// Logs in: opens the key of the account with the password, and its
/// database with that key. Returns the account and its key, which
/// the session keeps until logout.
///
/// Called by: `views/users/users.rs` (the login, the setup).
/// Calls: `system::read::user`, `keys::unwrap`, `user_db::open`.
pub fn log_in(
    system_db: &SystemDb,
    data_dir: &Path,
    name: &str,
    password: &str,
) -> Result<(Account, DatabaseKey), AccountError> {
    let Some(user) = system::read::user(system_db, name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, password, &binding(&user.name))?;

    // The database must open with the key; otherwise the account
    // is damaged, which is reported as such.
    user_db::open(data_dir, user.id, &key)?;

    Ok((Account { id: user.id, name: user.name }, key))
}

/// Changes the password of an account. The old password is needed: it
/// opens the key, which is then wrapped with the new one and with a new
/// recovery code; the old code stops working (user, 2026-10-24). The
/// database itself is not touched. Gives back the new code.
///
/// Called by: `views/users/users.rs` (`password_form_change`).
/// Calls: `keys::unwrap`, [`wrap_twice`], `system::update::replace_user_keys`.
pub fn change_password(
    system_db: &SystemDb,
    name: &str,
    old_password: &str,
    new_password: &str,
) -> Result<RecoveryCode, AccountError> {
    keys::check_password(new_password)?;

    let Some(user) = system::read::user(system_db, name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, old_password, &binding(&user.name))?;
    let (wrapped, recovery, code) = wrap_twice(&key, new_password, &user.name)?;

    system::update::replace_user_keys(system_db, user.id, &wrapped, &recovery)?;

    Ok(code)
}

/// The recovery of a forgotten password: the recovery code opens the key,
/// which is wrapped with the new password and a new recovery code (the old
/// code stops working). A wrong code and an unknown name give the same
/// answer as a wrong password. Gives back the account and its key, to log
/// in, and the new code.
///
/// Called by: `views/users/users.rs` (`login_form_recover`).
/// Calls: `recovery::unwrap`, `user_db::open`, [`wrap_twice`],
/// `system::update::replace_user_keys`.
pub fn recover(
    system_db: &SystemDb,
    data_dir: &Path,
    name: &str,
    code: &RecoveryCode,
    new_password: &str,
) -> Result<(Account, DatabaseKey, RecoveryCode), AccountError> {
    keys::check_password(new_password)?;

    let Some(user) = system::read::user(system_db, name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = recovery::unwrap(&user.recovery, code, &binding(&user.name))?;

    // The key must open the database before the old keys are replaced.
    user_db::open(data_dir, user.id, &key)?;

    let (wrapped, recovery, new_code) = wrap_twice(&key, new_password, &user.name)?;

    system::update::replace_user_keys(system_db, user.id, &wrapped, &recovery)?;

    Ok((Account { id: user.id, name: user.name }, key, new_code))
}

/// Makes a new recovery code for an account; the password is needed (user,
/// 2026-10-24) and the old code stops working. Gives back the new code.
///
/// Called by: `views/users/users.rs` (`recovery_code_form_replace`).
/// Calls: `keys::unwrap`, `recovery::new_code`, `recovery::wrap`,
/// `system::update::replace_recovery_key`.
pub fn replace_recovery_code(system_db: &SystemDb, name: &str, password: &str) -> Result<RecoveryCode, AccountError> {
    let Some(user) = system::read::user(system_db, name)? else {
        return Err(AccountError::WrongCredentials);
    };

    let key = keys::unwrap(&user.key, password, &binding(&user.name))?;
    let code = recovery::new_code()?;
    let recovery = recovery::wrap(&key, &code, &binding(&user.name))?;

    system::update::replace_recovery_key(system_db, user.id, &recovery)?;

    Ok(code)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `accounts.rs`.

    use super::*;
    use crate::db::system;

    use crate::model_old::installation::Installation;
    use crate::test_support::TempDir;

    const PASSWORD: &str = "correct horse battery";

    /// A system database in a temporary directory.
    fn setup() -> (TempDir, SystemDb) {
        let directory = TempDir::new();
        let installation = Installation { id: [1; 16] };
        let db = SystemDb::create(directory.path(), &installation).unwrap();

        (directory, db)
    }

    #[test]
    fn a_created_account_logs_in_and_opens_its_database() {
        let (directory, db) = setup();

        let (created, _) = create_first(&db, directory.path(), "Anna", PASSWORD).unwrap();
        let (account, key) = log_in(&db, directory.path(), "anna", PASSWORD).unwrap();

        // The name as it was given, whatever was typed at login.
        assert_eq!(account, created);
        assert_eq!(account.name, "Anna");
        assert!(user_db::open(directory.path(), account.id, &key).is_ok());
    }

    #[test]
    fn a_wrong_password_and_an_unknown_name_look_the_same() {
        let (directory, db) = setup();

        create_first(&db, directory.path(), "anna", PASSWORD).unwrap();

        assert!(matches!(
            log_in(&db, directory.path(), "anna", "wrong horse battery"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(matches!(log_in(&db, directory.path(), "nobody", PASSWORD), Err(AccountError::WrongCredentials)));
    }

    #[test]
    fn names_and_passwords_are_checked() {
        let (directory, db) = setup();

        assert!(matches!(create_first(&db, directory.path(), " ", PASSWORD), Err(AccountError::InvalidName(_))));
        assert!(matches!(create_first(&db, directory.path(), "jan", "short"), Err(AccountError::WeakPassword(_))));

        // Refused accounts leave nothing behind.
        assert!(system::read::user_names(&db).unwrap().is_empty());

        create_first(&db, directory.path(), "anna", PASSWORD).unwrap();
        assert!(matches!(create_first(&db, directory.path(), "ANNA", PASSWORD), Err(AccountError::NameTaken)));
    }

    #[test]
    fn a_changed_password_opens_the_same_database() {
        let (directory, db) = setup();

        create_first(&db, directory.path(), "anna", PASSWORD).unwrap();
        let (_, before) = log_in(&db, directory.path(), "anna", PASSWORD).unwrap();

        change_password(&db, "anna", PASSWORD, "a brand new password").unwrap();

        let (_, after) = log_in(&db, directory.path(), "anna", "a brand new password").unwrap();
        assert_eq!(before.expose(), after.expose());
        assert!(log_in(&db, directory.path(), "anna", PASSWORD).is_err());
    }

    #[test]
    fn a_password_is_changed_only_with_the_current_one() {
        let (directory, db) = setup();

        create_first(&db, directory.path(), "anna", PASSWORD).unwrap();

        assert!(matches!(
            change_password(&db, "anna", "not the password", "a brand new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(matches!(change_password(&db, "anna", PASSWORD, "short"), Err(AccountError::WeakPassword(_))));
        assert!(log_in(&db, directory.path(), "anna", PASSWORD).is_ok());
    }

    #[test]
    fn the_recovery_code_of_the_setup_sets_a_new_password() {
        let (directory, db) = setup();
        let (_, code) = create_first(&db, directory.path(), "anna", PASSWORD).unwrap();
        let (_, before) = log_in(&db, directory.path(), "anna", PASSWORD).unwrap();

        let (account, key, new_code) = recover(&db, directory.path(), "ANNA", &code, "a brand new password").unwrap();

        assert_eq!(account.name, "anna");
        assert_eq!(key.expose(), before.expose());
        assert!(log_in(&db, directory.path(), "anna", "a brand new password").is_ok());
        assert!(log_in(&db, directory.path(), "anna", PASSWORD).is_err());

        // The used code stops working; the new one works.
        assert!(matches!(
            recover(&db, directory.path(), "anna", &code, "another new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(recover(&db, directory.path(), "anna", &new_code, "another new password").is_ok());
    }

    #[test]
    fn a_change_of_the_password_makes_a_new_code() {
        let (directory, db) = setup();
        let (_, first) = create_first(&db, directory.path(), "anna", PASSWORD).unwrap();

        let second = change_password(&db, "anna", PASSWORD, "a brand new password").unwrap();

        assert_ne!(first.expose(), second.expose());
        assert!(matches!(
            recover(&db, directory.path(), "anna", &first, "another new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(recover(&db, directory.path(), "anna", &second, "another new password").is_ok());
    }

    #[test]
    fn a_new_code_needs_the_password_and_replaces_the_old_one() {
        let (directory, db) = setup();
        let (_, first) = create_first(&db, directory.path(), "anna", PASSWORD).unwrap();

        assert!(matches!(replace_recovery_code(&db, "anna", "not the password"), Err(AccountError::WrongCredentials)));

        let second = replace_recovery_code(&db, "anna", PASSWORD).unwrap();

        // The password still opens; only the code changed.
        assert!(log_in(&db, directory.path(), "anna", PASSWORD).is_ok());
        assert!(matches!(
            recover(&db, directory.path(), "anna", &first, "another new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(recover(&db, directory.path(), "anna", &second, "another new password").is_ok());
    }

    #[test]
    fn a_recovery_with_a_wrong_code_or_name_or_a_weak_password_changes_nothing() {
        let (directory, db) = setup();
        let (_, code) = create_first(&db, directory.path(), "anna", PASSWORD).unwrap();
        let other = recovery::new_code().unwrap();

        assert!(matches!(
            recover(&db, directory.path(), "anna", &other, "a brand new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(matches!(
            recover(&db, directory.path(), "nobody", &code, "a brand new password"),
            Err(AccountError::WrongCredentials)
        ));
        assert!(matches!(recover(&db, directory.path(), "anna", &code, "short"), Err(AccountError::WeakPassword(_))));
        assert!(log_in(&db, directory.path(), "anna", PASSWORD).is_ok());
    }
}
