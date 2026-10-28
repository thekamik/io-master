//! Reading the AI connections, the active one, and the API key of a
//! connection. The key is given out only as an [`ApiKey`], which is
//! wiped from memory and never shown (`model/api_key.rs`).

use crate::db_old::user;
use rusqlite::OptionalExtension;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::ai::StoredConnection;
use crate::model_old::api_key::ApiKey;

use super::rows::{ACTIVE, connection_from_row, key_name};

/// Whether a key is stored for a connection.
pub(super) fn has_key(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(user::read::secret(db, &key_name(id))?.is_some())
}

/// Every connection, by name.
pub fn list(db: &UserDb) -> Result<Vec<StoredConnection>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM ai_connections ORDER BY name COLLATE NOCASE, id")?;
    let rows = statement.query_map([], connection_from_row)?.collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(|(id, connection)| Ok(StoredConnection { id, connection, has_key: has_key(db, id)? }))
        .collect()
}

/// One connection, or `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<StoredConnection>, DbError> {
    let row = db
        .connection()
        .query_row("SELECT * FROM ai_connections WHERE id = ?1", [id], connection_from_row)
        .optional()?;

    match row {
        Some((id, connection)) => Ok(Some(StoredConnection { id, connection, has_key: has_key(db, id)? })),
        None => Ok(None),
    }
}

/// The number of the connection with `name` (letter case aside), if any.
pub(super) fn named(db: &UserDb, name: &str) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT id FROM ai_connections WHERE name = ?1 COLLATE NOCASE", [name], |row| row.get(0))
        .optional()?)
}

/// The address the API key of a connection is bound to: `None` when
/// there is no such connection, `Some(None)` when it has no key bound.
pub(super) fn key_origin(db: &UserDb, id: i64) -> Result<Option<Option<String>>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT key_origin FROM ai_connections WHERE id = ?1", [id], |row| row.get(0))
        .optional()?)
}

/// The active connection, or `None` when none is chosen (or the
/// chosen one no longer exists).
pub fn active(db: &UserDb) -> Result<Option<i64>, DbError> {
    let Some(id) = user::read::setting(db, ACTIVE)?.and_then(|text| text.parse::<i64>().ok()) else {
        return Ok(None);
    };

    Ok(get(db, id)?.map(|stored| stored.id))
}

/// The API key of a connection, or `None` when none is stored.
///
/// **The key is given out only for the address it was entered for**
/// (`update.rs`): it is checked here, at every use, against the
/// address the connection has now, so that no failed or interrupted
/// save can ever send it to another service. A key bound elsewhere,
/// or a stored text that is no longer a valid key (impossible unless
/// the database was changed by hand), counts as no key.
pub fn api_key(db: &UserDb, id: i64) -> Result<Option<ApiKey>, DbError> {
    let (Some(stored), Some(bound_to)) = (get(db, id)?, key_origin(db, id)?.flatten()) else {
        return Ok(None);
    };

    if stored.connection.origin().as_deref() != Some(bound_to.as_str()) {
        return Ok(None);
    }

    Ok(user::read::secret(db, &key_name(id))?.and_then(|text| ApiKey::new(text.as_str().to_owned()).ok()))
}

#[cfg(test)]
mod tests {
    //! Tests of reading the AI connections, the active one and the API key.

    use super::*;

    use crate::db_old::ai::connection::{create, update};
    use crate::test_support::db::{SECRET_KEY, key, named};
    use crate::test_support::standards::database;

    #[test]
    fn a_stored_connection_is_read_and_a_missing_one_is_none() {
        let test = database();
        create::connection(&test.db, &named("Local")).unwrap();

        assert_eq!(get(&test.db, 1).unwrap().unwrap().connection, named("Local"));
        assert_eq!(list(&test.db).unwrap().len(), 1);
        assert!(get(&test.db, 7).unwrap().is_none());
    }

    #[test]
    fn no_connection_is_active_before_the_first_is_stored() {
        let test = database();

        assert_eq!(active(&test.db).unwrap(), None);
    }

    #[test]
    fn a_stored_key_is_given_out_and_a_missing_one_is_none() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;

        assert!(api_key(&test.db, id).unwrap().is_none());

        update::api_key(&test.db, id, &key()).unwrap();
        assert_eq!(api_key(&test.db, id).unwrap().unwrap().expose(), SECRET_KEY);
        assert!(get(&test.db, id).unwrap().unwrap().has_key);
    }

    #[test]
    fn a_key_is_given_out_only_for_the_address_it_is_bound_to() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;
        update::api_key(&test.db, id, &key()).unwrap();
        assert!(api_key(&test.db, id).unwrap().is_some());

        // A save interrupted after the address changed, before the key was
        // removed: the key stays in the file, but is never given out.
        test.db
            .connection_for_tests()
            .execute("UPDATE ai_connections SET url = 'https://elsewhere.example/v1/chat' WHERE id = ?1", [id])
            .unwrap();
        assert!(get(&test.db, id).unwrap().unwrap().has_key);
        assert!(api_key(&test.db, id).unwrap().is_none());

        // A key with no recorded address is not given out either.
        test.db
            .connection_for_tests()
            .execute("UPDATE ai_connections SET key_origin = NULL WHERE id = ?1", [id])
            .unwrap();
        assert!(api_key(&test.db, id).unwrap().is_none());
    }
}
