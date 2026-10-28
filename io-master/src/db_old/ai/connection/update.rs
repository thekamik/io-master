//! Changing an AI connection, the active one, and the API key.
//!
//! **An API key is bound to the address it was entered for**: the
//! scheme, host and port of the URL ([`Connection::origin`]). When a
//! connection is saved with another address, its key is removed, so
//! that a changed URL never sends the key to another service.

use crate::db_old::user;
use rusqlite::named_params;
use rusqlite::types::ToSql;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::ai::{Connection, SavedConnection};
use crate::model_old::api_key::ApiKey;

use super::delete;
use super::read;
use super::rows::{ACTIVE, COLUMNS, SaveError, key_name, with_connection_values};

/// Changes connection `id`. Its name must stay free of the others;
/// the key is removed when the address changed. One transaction: a new
/// address is never stored while the key bound to the old one stays.
pub fn connection(db: &UserDb, id: i64, connection: &Connection) -> Result<SavedConnection, SaveError> {
    if read::named(db, &connection.name)?.is_some_and(|other| other != id) {
        return Err(SaveError::NameTaken);
    }

    let Some(origin) = read::key_origin(db, id)? else {
        return Err(SaveError::NoSuchConnection);
    };

    let transaction = db.connection().unchecked_transaction().map_err(DbError::from)?;

    with_connection_values(connection, |values| {
        transaction.execute(
            &format!(
                "UPDATE ai_connections SET {} WHERE id = :id",
                COLUMNS.map(|column| format!("{column} = :{column}")).join(", ")
            ),
            &[values, &[(":id", &id as &dyn ToSql)]].concat()[..],
        )
    })
    .map_err(|error| SaveError::Db(error.into()))?;

    // The key stays only with the address it was entered for.
    let moved = read::has_key(db, id)? && origin != connection.origin();

    if moved {
        delete::remove_key(db, id)?;
    }

    if read::active(db)?.is_none() {
        user::update::set_setting(db, ACTIVE, &id.to_string())?;
    }

    transaction.commit().map_err(DbError::from)?;

    Ok(SavedConnection { id, key_removed: moved })
}

/// Makes a connection the active one; returns whether it exists.
pub fn active(db: &UserDb, id: i64) -> Result<bool, DbError> {
    if read::get(db, id)?.is_none() {
        return Ok(false);
    }

    user::update::set_setting(db, ACTIVE, &id.to_string())?;

    Ok(true)
}

/// Stores the API key of a connection, bound to its address; returns
/// whether the connection exists. One transaction: the key and the
/// address it is bound to are stored together.
pub fn api_key(db: &UserDb, id: i64, key: &ApiKey) -> Result<bool, DbError> {
    let Some(stored) = read::get(db, id)? else {
        return Ok(false);
    };

    let transaction = db.connection().unchecked_transaction()?;

    user::update::set_secret(db, &key_name(id), key.expose())?;
    transaction.execute(
        "UPDATE ai_connections SET key_origin = :key_origin WHERE id = :id",
        named_params! { ":key_origin": stored.connection.origin(), ":id": id },
    )?;
    transaction.commit()?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    //! Tests of changing an AI connection, the active one and the API key.

    use super::*;

    use std::fs;

    use crate::db_old::ai::connection::{create, read};
    use crate::db_old::user::database as user_db;
    use crate::model_old::ai::AnswerMode;
    use crate::test_support::db::{SECRET_KEY, key, named};
    use crate::test_support::standards::database;

    #[test]
    fn a_connection_is_changed() {
        let test = database();
        create::connection(&test.db, &named("Local")).unwrap();

        let changed = Connection {
            answer: AnswerMode::Whole,
            text_path: "choices[0].message.content".to_owned(),
            ..named("Local, whole")
        };
        connection(&test.db, 1, &changed).unwrap();

        assert_eq!(read::get(&test.db, 1).unwrap().unwrap().connection, changed);
    }

    #[test]
    fn a_name_of_another_connection_is_refused_and_its_own_is_kept() {
        let test = database();
        create::connection(&test.db, &named("Local")).unwrap();
        let second = create::connection(&test.db, &named("Other")).unwrap().id;

        assert!(matches!(connection(&test.db, second, &named("LOCAL")), Err(SaveError::NameTaken)));
        connection(&test.db, second, &named("Other")).unwrap();
    }

    #[test]
    fn a_connection_that_does_not_exist_is_not_changed() {
        let test = database();

        assert!(matches!(connection(&test.db, 99, &named("New")), Err(SaveError::NoSuchConnection)));
    }

    #[test]
    fn another_connection_can_be_made_the_active_one() {
        let test = database();
        create::connection(&test.db, &named("A")).unwrap();
        let second = create::connection(&test.db, &named("B")).unwrap().id;

        assert!(active(&test.db, second).unwrap());
        assert_eq!(read::active(&test.db).unwrap(), Some(second));
        assert!(!active(&test.db, 99).unwrap());
    }

    #[test]
    fn a_key_is_stored_only_for_a_connection_that_exists() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;

        assert!(api_key(&test.db, id, &key()).unwrap());
        assert!(read::api_key(&test.db, id).unwrap().is_some());
        assert!(!api_key(&test.db, 99, &key()).unwrap());
    }

    #[test]
    fn a_changed_address_removes_the_key() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;
        api_key(&test.db, id, &key()).unwrap();

        // Another path at the same address keeps the key.
        let other_path = Connection { url: "http://127.0.0.1:11434/api/chat".to_owned(), ..named("Local") };
        assert!(!connection(&test.db, id, &other_path).unwrap().key_removed);
        assert!(read::api_key(&test.db, id).unwrap().is_some());

        // Another port is another address.
        let other_port = Connection { url: "http://127.0.0.1:8080/api/chat".to_owned(), ..named("Local") };
        assert!(connection(&test.db, id, &other_port).unwrap().key_removed);
        assert!(read::api_key(&test.db, id).unwrap().is_none());
    }

    #[test]
    fn a_failed_change_leaves_nothing_behind() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;
        api_key(&test.db, id, &key()).unwrap();

        // The last statement of the save fails: the database refuses to
        // store the active connection.
        test.db
            .connection_for_tests()
            .execute_batch(
                "DELETE FROM settings WHERE name = 'ai.active_connection';
             CREATE TRIGGER refuse BEFORE INSERT ON settings BEGIN SELECT RAISE(ABORT, 'refused'); END;",
            )
            .unwrap();

        let elsewhere = Connection { url: "http://127.0.0.1:8080/api/chat".to_owned(), ..named("Moved") };
        assert!(connection(&test.db, id, &elsewhere).is_err());

        // Nothing of it was stored: the old address, and its key with it.
        assert_eq!(read::get(&test.db, id).unwrap().unwrap().connection, named("Local"));
        assert!(read::api_key(&test.db, id).unwrap().is_some());
    }

    #[test]
    fn the_key_cannot_be_read_from_the_file() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;
        api_key(&test.db, id, &key()).unwrap();
        let directory = test.directory;
        drop(test.db);

        let bytes = fs::read(user_db::path(directory.path(), 1)).unwrap();
        assert!(!bytes.windows(SECRET_KEY.len()).any(|window| window == SECRET_KEY.as_bytes()));
    }
}
