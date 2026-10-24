//! Reading the system database: the installation and the accounts.

use rusqlite::OptionalExtension;

use crate::db::error::{Corrupt, DbError};
use crate::model::installation::{ID_LENGTH, Installation, InstallationId};

use super::database::SystemDb;
use super::rows::{UserRecord, user_from_row};

/// The record of the installation.
pub fn installation(db: &SystemDb) -> Result<Installation, DbError> {
    let id: Vec<u8> = db.connection().query_row("SELECT id FROM installation", [], |row| row.get(0))?;

    let id: InstallationId = id.try_into().map_err(|id: Vec<u8>| {
        DbError::Corrupt(Corrupt {
            table: "installation",
            column: "id",
            value: format!("{} bytes, not {ID_LENGTH}", id.len()),
        })
    })?;

    Ok(Installation { id })
}

/// The names of all accounts, the oldest first. Used by the tests only:
/// the program knows its one account by its name at the login.
#[cfg(test)]
pub fn user_names(db: &SystemDb) -> Result<Vec<String>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT name FROM users ORDER BY id")?;

    let names = statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;

    Ok(names)
}

/// The account with the given name, letter case aside.
pub fn user(db: &SystemDb, name: &str) -> Result<Option<UserRecord>, DbError> {
    Ok(db.connection().query_row("SELECT * FROM users WHERE name = ?1", [name], user_from_row).optional()?)
}

#[cfg(test)]
mod tests {
    //! Tests of reading the installation and the accounts.

    use super::*;

    use crate::db::system::create;
    use crate::test_support::TempDir;
    use crate::test_support::db::{recovery_key, system_db, wrapped_key};

    #[test]
    fn the_names_are_read_in_the_order_they_were_added() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        create::add_user(&db, "Anna", &wrapped_key(), &recovery_key()).unwrap();
        create::add_user(&db, "jan", &wrapped_key(), &recovery_key()).unwrap();

        assert_eq!(user_names(&db).unwrap(), ["Anna", "jan"]);
    }

    #[test]
    fn an_account_is_found_without_regard_to_letter_case() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let anna = create::add_user(&db, "Anna", &wrapped_key(), &recovery_key()).unwrap();

        let found = user(&db, "anna").unwrap().unwrap();
        assert_eq!((found.id, found.name.as_str()), (anna, "Anna"));
        assert_eq!(user(&db, "nobody").unwrap(), None);
    }
}
