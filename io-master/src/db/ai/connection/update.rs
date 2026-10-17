//! Changing an AI connection, the active one, and the API key.
//!
//! **An API key is bound to the address it was entered for**: the
//! scheme, host and port of the URL ([`Connection::origin`]). When a
//! connection is saved with another address, its key is removed, so
//! that a changed URL never sends the key to another service.

use crate::db::user;
use rusqlite::named_params;
use rusqlite::types::ToSql;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::{Connection, SavedConnection};
use crate::model::api_key::ApiKey;

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
