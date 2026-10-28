//! Deleting an AI connection and its API key.

use crate::db_old::error::DbError;
use crate::db_old::user;
use crate::db_old::user::database::UserDb;

use super::rows::{ACTIVE, key_name};

/// Deletes a connection with its key; returns whether it existed.
/// One transaction.
pub fn connection(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    remove_key(db, id)?;

    let deleted = transaction.execute("DELETE FROM ai_connections WHERE id = ?1", [id])? > 0;

    if deleted && user::read::setting(db, ACTIVE)?.as_deref() == Some(id.to_string().as_str()) {
        user::delete::remove_setting(db, ACTIVE)?;
    }

    transaction.commit()?;

    Ok(deleted)
}

/// Removes the API key of a connection (overwritten in the file). One
/// transaction.
pub fn api_key(db: &UserDb, id: i64) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    remove_key(db, id)?;
    transaction.commit()?;

    Ok(())
}

/// Removes the key and the address it was bound to, inside the
/// transaction of the caller (`connection`, `api_key`, and
/// `update::connection` when the address changed).
pub(super) fn remove_key(db: &UserDb, id: i64) -> Result<(), DbError> {
    user::delete::remove_secret(db, &key_name(id))?;
    db.connection().execute("UPDATE ai_connections SET key_origin = NULL WHERE id = ?1", [id])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of deleting an AI connection and its API key.

    use super::*;

    use crate::db_old::ai::connection::{create, read, update};
    use crate::test_support::db::{key, named};
    use crate::test_support::standards::database;

    #[test]
    fn deleting_the_active_connection_leaves_none_active() {
        let test = database();
        create::connection(&test.db, &named("A")).unwrap();
        let second = create::connection(&test.db, &named("B")).unwrap().id;
        update::active(&test.db, second).unwrap();

        assert!(connection(&test.db, second).unwrap());
        assert_eq!(read::active(&test.db).unwrap(), None);
        assert!(!connection(&test.db, second).unwrap());
    }

    #[test]
    fn the_key_is_removed_with_its_connection() {
        let test = database();
        let id = create::connection(&test.db, &named("Local")).unwrap().id;
        update::api_key(&test.db, id, &key()).unwrap();

        connection(&test.db, id).unwrap();
        assert!(read::api_key(&test.db, id).unwrap().is_none());
    }
}
