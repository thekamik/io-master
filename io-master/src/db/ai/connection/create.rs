//! Storing a new AI connection. Its fields are checked by
//! `ai/api/connections.rs` before; here the rules of the stored data:
//! a name of its own, and the first connection becomes the active one.

use crate::db::error::DbError;
use crate::db::user;
use crate::db::user::database::UserDb;
use crate::model_old::ai::{Connection, SavedConnection};

use super::read;
use super::rows::{ACTIVE, COLUMNS, SaveError, with_connection_values};

/// Stores a new connection. Its name must be free; the first
/// connection becomes the active one. One transaction: the row and
/// the choice of the active connection are stored together or not at
/// all.
pub fn connection(db: &UserDb, connection: &Connection) -> Result<SavedConnection, SaveError> {
    if read::named(db, &connection.name)?.is_some() {
        return Err(SaveError::NameTaken);
    }

    let transaction = db.connection().unchecked_transaction().map_err(DbError::from)?;

    with_connection_values(connection, |values| {
        transaction.execute(
            &format!(
                "INSERT INTO ai_connections ({}) VALUES ({})",
                COLUMNS.join(", "),
                COLUMNS.map(|column| format!(":{column}")).join(", ")
            ),
            values,
        )
    })
    .map_err(|error| SaveError::Db(error.into()))?;

    let id = transaction.last_insert_rowid();

    if read::active(db)?.is_none() {
        user::update::set_setting(db, ACTIVE, &id.to_string())?;
    }

    transaction.commit().map_err(DbError::from)?;

    Ok(SavedConnection { id, key_removed: false })
}

#[cfg(test)]
mod tests {
    //! Tests of storing a new AI connection.

    use super::*;

    use crate::db::ai::connection::read;
    use crate::test_support::db::named;
    use crate::test_support::standards::database;

    #[test]
    fn a_connection_is_stored_with_the_next_id_and_no_key() {
        let test = database();

        let saved = connection(&test.db, &named("Local")).unwrap();
        assert_eq!(saved, SavedConnection { id: 1, key_removed: false });

        let stored = read::get(&test.db, 1).unwrap().unwrap();
        assert_eq!(stored.connection, named("Local"));
        assert!(!stored.has_key);
    }

    #[test]
    fn a_name_taken_in_another_case_is_refused() {
        let test = database();
        connection(&test.db, &named("Local")).unwrap();

        assert!(matches!(connection(&test.db, &named("local")), Err(SaveError::NameTaken)));
    }

    #[test]
    fn the_first_connection_becomes_the_active_one() {
        let test = database();
        let first = connection(&test.db, &named("A")).unwrap().id;
        connection(&test.db, &named("B")).unwrap();

        assert_eq!(read::active(&test.db).unwrap(), Some(first));
    }

    #[test]
    fn a_connection_the_database_refuses_leaves_nothing_behind() {
        let test = database();
        connection(&test.db, &named("Local")).unwrap();

        // The last statement of the save fails: the database refuses to
        // store the active connection.
        test.db
            .connection_for_tests()
            .execute_batch(
                "DELETE FROM settings WHERE name = 'ai.active_connection';
             CREATE TRIGGER refuse BEFORE INSERT ON settings BEGIN SELECT RAISE(ABORT, 'refused'); END;",
            )
            .unwrap();

        assert!(connection(&test.db, &named("Second")).is_err());
        assert_eq!(read::list(&test.db).unwrap().len(), 1);
    }
}
