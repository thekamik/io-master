//! Creating rows of the system database: the record of the
//! installation (once, with the database) and the accounts.

use rusqlite::{Connection, named_params};

use crate::db_old::encryption::keys::WrappedKey;
use crate::db_old::encryption::recovery::RecoveryWrappedKey;
use crate::db_old::error::DbError;
use crate::model_old::installation::Installation;
use crate::model_old::time::now;

use super::database::SystemDb;

/// Writes the record of the installation into a new database
/// (`SystemDb::create`).
pub(super) fn installation(connection: &Connection, installation: &Installation) -> Result<(), DbError> {
    connection.execute(
        "INSERT INTO installation (id, created) VALUES (:id, :created)",
        named_params! { ":id": installation.id.as_slice(), ":created": now() },
    )?;

    Ok(())
}

/// Adds an account with the key of its database wrapped twice (with the
/// password and with the recovery code) and returns its id. Fails when
/// the name is taken.
///
/// Called by: `installation/accounts.rs` (`create`).
pub fn add_user(db: &SystemDb, name: &str, key: &WrappedKey, recovery: &RecoveryWrappedKey) -> Result<i64, DbError> {
    let connection = db.connection();

    connection.execute(
        "INSERT INTO users (name, created, key_salt, key_memory_kib, key_iterations, key_nonce, key_ciphertext, key_tag,
                            recovery_salt, recovery_nonce, recovery_ciphertext, recovery_tag)
         VALUES (:name, :created, :key_salt, :key_memory_kib, :key_iterations, :key_nonce, :key_ciphertext, :key_tag,
                 :recovery_salt, :recovery_nonce, :recovery_ciphertext, :recovery_tag)",
        named_params! {
            ":name": name,
            ":created": now(),
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

    Ok(connection.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    //! Tests of adding an account to the system database.

    use super::*;

    use crate::db_old::system::read;
    use crate::test_support::TempDir;
    use crate::test_support::db::{recovery_key, system_db, wrapped_key};

    #[test]
    fn an_account_is_stored_with_its_key() {
        let directory = TempDir::new();
        let db = system_db(&directory);

        let id = add_user(&db, "Anna", &wrapped_key(), &recovery_key()).unwrap();

        let found = read::user(&db, "Anna").unwrap().unwrap();
        assert_eq!((found.id, found.name.as_str(), &found.key), (id, "Anna", &wrapped_key()));
        assert_eq!(found.recovery, recovery_key());
    }

    #[test]
    fn a_name_is_taken_once_whatever_its_case() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        add_user(&db, "Anna", &wrapped_key(), &recovery_key()).unwrap();

        assert!(add_user(&db, "ANNA", &wrapped_key(), &recovery_key()).is_err());
    }
}
