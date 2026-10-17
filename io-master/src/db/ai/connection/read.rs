//! Reading the AI connections, the active one, and the API key of a
//! connection. The key is given out only as an [`ApiKey`], which is
//! wiped from memory and never shown (`model/api_key.rs`).

use crate::db::user;
use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::api_key::ApiKey;

use super::rows::{ACTIVE, Stored, connection_from_row, key_name};

/// Whether a key is stored for a connection.
pub(super) fn has_key(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(user::read::secret(db, &key_name(id))?.is_some())
}

/// Every connection, by name.
pub fn list(db: &UserDb) -> Result<Vec<Stored>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM ai_connections ORDER BY name COLLATE NOCASE, id")?;
    let rows = statement.query_map([], connection_from_row)?.collect::<Result<Vec<_>, _>>()?;

    rows.into_iter().map(|(id, connection)| Ok(Stored { id, connection, has_key: has_key(db, id)? })).collect()
}

/// One connection, or `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Stored>, DbError> {
    let row = db
        .connection()
        .query_row("SELECT * FROM ai_connections WHERE id = ?1", [id], connection_from_row)
        .optional()?;

    match row {
        Some((id, connection)) => Ok(Some(Stored { id, connection, has_key: has_key(db, id)? })),
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
