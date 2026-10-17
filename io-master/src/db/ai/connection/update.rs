//! Changing an AI connection, the active one, and the API key.
//!
//! **An API key is bound to the address it was entered for**: the
//! scheme, host and port of the URL ([`Connection::origin`]). When a
//! connection is saved with another address, its key is removed, so
//! that a changed URL never sends the key to another service.

use rusqlite::params;
use rusqlite::types::ToSql;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Connection;
use crate::model::api_key::ApiKey;

use super::delete;
use super::read;
use super::rows::{ACTIVE, SaveError, Saved, key_name, with_connection_values};

/// Changes connection `id`. Its name must stay free of the others;
/// the key is removed when the address changed. One transaction: a new
/// address is never stored while the key bound to the old one stays.
pub fn connection(db: &UserDb, id: i64, connection: &Connection) -> Result<Saved, SaveError> {
    connection.check().map_err(SaveError::Invalid)?;

    if read::named(db, connection.name.trim())?.is_some_and(|other| other != id) {
        return Err(SaveError::NameTaken);
    }

    let Some(origin) = read::key_origin(db, id)? else {
        return Err(SaveError::NoSuchConnection);
    };

    let transaction = db.connection().unchecked_transaction().map_err(DbError::from)?;

    with_connection_values(connection, |values| {
        transaction.execute(
            "UPDATE ai_connections SET name = ?1, url = ?2, model = ?3, ca_certificate = ?4, headers = ?5, \
             body = ?6, user_role = ?7, model_role = ?8, system_role = ?9, system_prompt = ?10, answer = ?11, \
             text_path = ?12, stop_path = ?13, cut_reasons = ?14, refused_reasons = ?15, error_path = ?16, \
             model_path = ?17, input_tokens_path = ?18, output_tokens_path = ?19, done_marker = ?20, \
             message_template = ?21 WHERE id = ?22",
            &[values, &[&id as &dyn ToSql]].concat()[..],
        )
    })
    .map_err(|error| SaveError::Db(error.into()))?;

    // The key stays only with the address it was entered for.
    let moved = read::has_key(db, id)? && origin != connection.origin();

    if moved {
        delete::remove_key(db, id)?;
    }

    if read::active(db)?.is_none() {
        db.set_setting(ACTIVE, &id.to_string())?;
    }

    transaction.commit().map_err(DbError::from)?;

    Ok(Saved { id, key_removed: moved })
}

/// Makes a connection the active one; returns whether it exists.
pub fn active(db: &UserDb, id: i64) -> Result<bool, DbError> {
    if read::get(db, id)?.is_none() {
        return Ok(false);
    }

    db.set_setting(ACTIVE, &id.to_string())?;

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

    db.set_secret(&key_name(id), key.expose())?;
    transaction
        .execute("UPDATE ai_connections SET key_origin = ?1 WHERE id = ?2", params![stored.connection.origin(), id])?;
    transaction.commit()?;

    Ok(true)
}
