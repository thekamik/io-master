//! Changing rows of the system database: the key of an account.

use rusqlite::named_params;

use crate::db::encryption::keys::WrappedKey;
use crate::db::error::DbError;

use super::database::SystemDb;

/// Stores a newly encrypted key of an account, after a change
/// of its password.
pub fn replace_user_key(db: &SystemDb, id: i64, key: &WrappedKey) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE users SET key_salt = :key_salt, key_memory_kib = :key_memory_kib, key_iterations = :key_iterations,
                          key_device = :key_device, key_nonce = :key_nonce, key_ciphertext = :key_ciphertext, key_tag = :key_tag
         WHERE id = :id",
        named_params! { ":id": id, ":key_salt": key.salt, ":key_memory_kib": key.memory_kib, ":key_iterations": key.iterations, ":key_device": key.with_device_secret, ":key_nonce": key.nonce, ":key_ciphertext": key.ciphertext, ":key_tag": key.tag },
    )?;

    if changed == 1 { Ok(()) } else { Err(DbError::NoRow { table: "users", id }) }
}

#[cfg(test)]
mod tests {
    //! Tests of replacing the key of an account.

    use super::*;

    use crate::db::system::{create, read};
    use crate::test_support::TempDir;
    use crate::test_support::db::{system_db, wrapped_key};

    #[test]
    fn the_key_of_an_account_is_replaced() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let id = create::add_user(&db, "anna", &wrapped_key()).unwrap();

        let mut new_key = wrapped_key();
        new_key.nonce = vec![9; 12];
        replace_user_key(&db, id, &new_key).unwrap();

        assert_eq!(read::user(&db, "anna").unwrap().unwrap().key, new_key);
    }

    #[test]
    fn the_key_of_an_account_that_does_not_exist_is_refused() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let id = create::add_user(&db, "anna", &wrapped_key()).unwrap();

        assert!(replace_user_key(&db, id + 1, &wrapped_key()).is_err());
    }
}
