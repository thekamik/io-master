//! The two copies of the key of a user made anew (`docs/architecture.md`,
//! section 5.4): after a change of the password, a recovery with the code,
//! or a new recovery code asked for. The key of the database stays the
//! same, so the database itself is not touched; only its two copies are
//! wrapped again, with the password and with a new recovery code, and
//! written over the old ones together.
//!
//! ```text
//! key of the database (opened by the password or the code)
//!   ──password (password::wrap)──► WrappedKey ─────────────┐
//!   ──new code (recovery::wrap)──► RecoveryWrappedKey ─────┴──► user_keys (update_user_keys)
//!   ◄── the new code, to be shown once; the old code stops working
//! ```

use std::fmt;

use crate::infrastructure::storage::encryption::database_key::DatabaseKey;
use crate::infrastructure::storage::error::DbError;
use crate::infrastructure::storage::system_database::SystemDatabase;
use crate::objects::user::user_keys::user_keys_model::UserKeys;
use crate::objects::user::user_keys::user_keys_rules::password::{self, KeyError};
use crate::objects::user::user_keys::user_keys_rules::recovery::{self, RecoveryError};
use crate::objects::user::user_keys::user_keys_rules::recovery_code::RecoveryCode;
use crate::objects::user::user_keys::user_keys_store::update_user_keys;

/// Why the keys could not be made anew. None of the variants holds a
/// password, a key or a code.
#[derive(Debug)]
pub enum RenewalError {
    /// The copy wrapped with the password (`password.rs`: `wrap`).
    Password(KeyError),
    /// The new code or the copy wrapped with it (`recovery.rs`: `new_code`,
    /// `wrap`).
    Recovery(RecoveryError),
    /// The system database (`user_keys_store.rs`: `update_user_keys`).
    Db(DbError),
}

impl fmt::Display for RenewalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenewalError::Password(error) => write!(formatter, "{error}"),
            RenewalError::Recovery(error) => write!(formatter, "{error}"),
            RenewalError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RenewalError {}

impl From<KeyError> for RenewalError {
    fn from(error: KeyError) -> RenewalError {
        RenewalError::Password(error)
    }
}

impl From<RecoveryError> for RenewalError {
    fn from(error: RecoveryError) -> RenewalError {
        RenewalError::Recovery(error)
    }
}

impl From<DbError> for RenewalError {
    fn from(error: DbError) -> RenewalError {
        RenewalError::Db(error)
    }
}

/// Wraps the key of the database of the user `user` anew with `password`
/// and with a new recovery code, writes both copies over the old ones, and
/// gives the new code ("ABCDE-FGHJK-...", shown once). From then on the old
/// code opens nothing, and neither does an old password.
///
/// Input:
/// - `system`: the open system database.
/// - `user`: the id of a user with keys (1).
/// - `key`: the key of their database, opened a moment before with their
///   password or their code.
/// - `password`: the password that is to open it from now on, checked by
///   the caller (`password.rs`: `check_password`).
/// - `binding`: what both copies are bound to, the id of the user
///   (`objects/user/user_data/user_data_rules.rs`: `binding`).
///
/// Output: the new recovery code; an error when a copy cannot be made, or
/// `RenewalError::Db` (`DbError::NoRow`) when the user has no keys.
///
/// Called by: `objects/user/user_data/user_data_rules.rs`: `recover`,
/// `change_password`, `new_recovery_code`.
/// Calls: `objects/user/user_keys/user_keys_rules/recovery.rs`: `new_code`, `wrap`;
/// `objects/user/user_keys/user_keys_rules/password.rs`: `wrap`; `objects/user/user_keys/user_keys_store.rs`:
/// `update_user_keys`.
pub fn renew_keys(
    system: &SystemDatabase,
    user: i64,
    key: &DatabaseKey,
    password: &str,
    binding: &[u8],
) -> Result<RecoveryCode, RenewalError> {
    let code = recovery::new_code()?;
    // `password::wrap` is the function of `password.rs`; `password` beside
    // it is the password the user gave.
    let keys = UserKeys {
        user,
        password: password::wrap(key, password, binding)?,
        recovery: recovery::wrap(key, &code, binding)?,
    };

    update_user_keys(system, &keys)?;

    Ok(code)
}

#[cfg(test)]
mod tests {
    //! Tests of the renewal: the new code and the password open the key, the
    //! old code no longer does.

    use super::*;

    use crate::infrastructure::storage::encryption::generate_key::generate_database_key;
    use crate::infrastructure::storage::system_database::create;
    use crate::objects::schema::SYSTEM_TABLES;
    use crate::objects::user::user_data::user_data_model::User;
    use crate::objects::user::user_data::user_data_store::create_user;
    use crate::objects::user::user_keys::user_keys_store::{create_user_keys, read_user_keys};
    use crate::test_support::TempDir;

    const BINDING: &[u8] = b"io-master user 1";

    #[test]
    fn the_new_code_and_the_password_open_the_key_and_the_old_code_does_not() {
        let directory = TempDir::new();
        let system = create(directory.path(), &SYSTEM_TABLES).unwrap();
        let user = create_user(&system, &User { id: 0, name: "Kamil".to_owned(), created: 1 }).unwrap();
        let key = generate_database_key().unwrap();
        let old = recovery::new_code().unwrap();
        let first = UserKeys {
            user,
            password: password::wrap(&key, "the first password", BINDING).unwrap(),
            recovery: recovery::wrap(&key, &old, BINDING).unwrap(),
        };

        create_user_keys(&system, &first).unwrap();

        let code = renew_keys(&system, user, &key, "the second password", BINDING).unwrap();
        let stored = read_user_keys(&system, user).unwrap();

        assert_eq!(recovery::unwrap(&stored.recovery, &code, BINDING).unwrap().expose(), key.expose());
        assert_eq!(password::unwrap(&stored.password, "the second password", BINDING).unwrap().expose(), key.expose());
        assert!(matches!(recovery::unwrap(&stored.recovery, &old, BINDING), Err(RecoveryError::WrongCode)));
        assert!(matches!(
            password::unwrap(&stored.password, "the first password", BINDING),
            Err(KeyError::WrongPassword)
        ));
    }

    #[test]
    fn a_user_without_keys_is_refused() {
        let directory = TempDir::new();
        let system = create(directory.path(), &SYSTEM_TABLES).unwrap();
        let key = generate_database_key().unwrap();

        assert!(matches!(
            renew_keys(&system, 1, &key, "a long enough password", BINDING),
            Err(RenewalError::Db(DbError::NoRow { .. }))
        ));
    }
}
