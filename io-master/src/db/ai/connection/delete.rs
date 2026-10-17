//! Deleting an AI connection and its API key.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::rows::{ACTIVE, key_name};

/// Deletes a connection with its key; returns whether it existed. A
/// role that used it uses the active connection again (`db/ai/roles/`,
/// `ON DELETE SET NULL`). One transaction.
pub fn connection(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    remove_key(db, id)?;

    let deleted = transaction.execute("DELETE FROM ai_connections WHERE id = ?1", [id])? > 0;

    if deleted && db.setting(ACTIVE)?.as_deref() == Some(id.to_string().as_str()) {
        db.remove_setting(ACTIVE)?;
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
    db.remove_secret(&key_name(id))?;
    db.connection().execute("UPDATE ai_connections SET key_origin = NULL WHERE id = ?1", [id])?;

    Ok(())
}
