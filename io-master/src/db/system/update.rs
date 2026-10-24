//! Changing rows of the system database: the keys of the account.

use rusqlite::named_params;

use crate::db::encryption::keys::WrappedKey;
use crate::db::encryption::recovery::RecoveryWrappedKey;
use crate::db::error::DbError;

use super::database::SystemDb;

/// Stores both copies of the key of an account anew, in one statement:
/// after a change of the password, which also makes a new recovery code,
/// so that the old code stops working (step 49 of docs/todo.md).
///
/// Called by: `installation/accounts.rs` (`change_password`, `recover`).
pub fn replace_user_keys(
    db: &SystemDb,
    id: i64,
    key: &WrappedKey,
    recovery: &RecoveryWrappedKey,
) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE users SET key_salt = :key_salt, key_memory_kib = :key_memory_kib, key_iterations = :key_iterations,
                          key_nonce = :key_nonce, key_ciphertext = :key_ciphertext, key_tag = :key_tag,
                          recovery_salt = :recovery_salt, recovery_nonce = :recovery_nonce,
                          recovery_ciphertext = :recovery_ciphertext, recovery_tag = :recovery_tag
         WHERE id = :id",
        named_params! {
            ":id": id,
            ":key_salt": key.salt,
            ":key_memory_kib": key.memory_kib,
            ":key_iterations": key.iterations,
            ":key_nonce": key.nonce,
            ":key_ciphertext": key.ciphertext,
            ":key_tag": key.tag,
            ":recovery_salt": recovery.salt,
            ":recovery_nonce": recovery.nonce,
            ":recovery_ciphertext": recovery.ciphertext,
            ":recovery_tag": recovery.tag,
        },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "users", id }) }
}

/// Stores a new copy of the key wrapped with a new recovery code; the old
/// code stops working.
///
/// Called by: `installation/accounts.rs` (`replace_recovery_code`).
pub fn replace_recovery_key(db: &SystemDb, id: i64, recovery: &RecoveryWrappedKey) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE users SET recovery_salt = :recovery_salt, recovery_nonce = :recovery_nonce,
                          recovery_ciphertext = :recovery_ciphertext, recovery_tag = :recovery_tag
         WHERE id = :id",
        named_params! {
            ":id": id,
            ":recovery_salt": recovery.salt,
            ":recovery_nonce": recovery.nonce,
            ":recovery_ciphertext": recovery.ciphertext,
            ":recovery_tag": recovery.tag,
        },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "users", id }) }
}

#[cfg(test)]
mod tests {
    //! Tests of replacing the keys of an account.

    use super::*;

    use crate::db::system::{create, read};
    use crate::test_support::TempDir;
    use crate::test_support::db::{recovery_key, system_db, wrapped_key};

    #[test]
    fn both_keys_of_an_account_are_replaced() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let id = create::add_user(&db, "anna", &wrapped_key(), &recovery_key()).unwrap();

        let mut new_key = wrapped_key();
        new_key.nonce = vec![9; 12];
        let mut new_recovery = recovery_key();
        new_recovery.nonce = vec![8; 12];
        replace_user_keys(&db, id, &new_key, &new_recovery).unwrap();

        let found = read::user(&db, "anna").unwrap().unwrap();
        assert_eq!((found.key, found.recovery), (new_key, new_recovery));
    }

    #[test]
    fn the_recovery_key_alone_is_replaced() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let id = create::add_user(&db, "anna", &wrapped_key(), &recovery_key()).unwrap();

        let mut new_recovery = recovery_key();
        new_recovery.salt = vec![7; 16];
        replace_recovery_key(&db, id, &new_recovery).unwrap();

        let found = read::user(&db, "anna").unwrap().unwrap();
        assert_eq!((found.key, found.recovery), (wrapped_key(), new_recovery));
    }

    #[test]
    fn the_keys_of_an_account_that_does_not_exist_are_refused() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let id = create::add_user(&db, "anna", &wrapped_key(), &recovery_key()).unwrap();

        assert!(replace_user_keys(&db, id + 1, &wrapped_key(), &recovery_key()).is_err());
        assert!(replace_recovery_key(&db, id + 1, &recovery_key()).is_err());
    }
}
