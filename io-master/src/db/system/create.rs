//! Creating rows of the system database: the record of the
//! installation (once, with the database) and the accounts.

use rusqlite::{Connection, named_params};

use crate::db::encryption::keys::WrappedKey;
use crate::db::error::DbError;
use crate::model::installation::Installation;
use crate::model::time::now;

use super::database::SystemDb;

/// Writes the record of the installation into a new database
/// (`SystemDb::create`).
pub(super) fn installation(connection: &Connection, installation: &Installation) -> Result<(), DbError> {
    connection.execute(
        "INSERT INTO installation (id, role, created) VALUES (:id, :role, :created)",
        named_params! { ":id": installation.id.as_slice(), ":role": installation.role.name(), ":created": now() },
    )?;

    Ok(())
}

/// Adds an account and returns its id. Fails when the name is
/// taken.
pub fn add_user(db: &SystemDb, name: &str, key: &WrappedKey) -> Result<i64, DbError> {
    let connection = db.connection();

    connection.execute(
        "INSERT INTO users (name, created, key_salt, key_memory_kib, key_iterations,
                            key_device, key_nonce, key_ciphertext, key_tag)
         VALUES (:name, :created, :key_salt, :key_memory_kib, :key_iterations, :key_device, :key_nonce, :key_ciphertext, :key_tag)",
        named_params! { ":name": name, ":created": now(), ":key_salt": key.salt, ":key_memory_kib": key.memory_kib, ":key_iterations": key.iterations, ":key_device": key.with_device_secret, ":key_nonce": key.nonce, ":key_ciphertext": key.ciphertext, ":key_tag": key.tag },
    )?;

    Ok(connection.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    //! Tests of adding an account to the system database.

    use super::*;

    use crate::db::system::read;
    use crate::test_support::TempDir;
    use crate::test_support::db::{system_db, wrapped_key};

    #[test]
    fn an_account_is_stored_with_its_key() {
        let directory = TempDir::new();
        let db = system_db(&directory);

        let id = add_user(&db, "Anna", &wrapped_key()).unwrap();

        let found = read::user(&db, "Anna").unwrap().unwrap();
        assert_eq!((found.id, found.name.as_str(), &found.key), (id, "Anna", &wrapped_key()));
    }

    #[test]
    fn a_name_is_taken_once_whatever_its_case() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        add_user(&db, "Anna", &wrapped_key()).unwrap();

        assert!(add_user(&db, "ANNA", &wrapped_key()).is_err());
    }
}
