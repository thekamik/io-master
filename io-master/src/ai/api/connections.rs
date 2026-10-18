//! The AI connections of the evaluator, as a resource (step 40): the
//! only way the rest of the program reaches them. A connection is
//! stored only in its normal form (`model/ai.rs`) and only when it
//! keeps the rules (`connection.rs`, [`Connection::check`]); whether
//! the service answers is not needed: a local model may be switched off
//! while it is set up, and the check of a connection is a step of its
//! own (`checks.rs`). The rules of the stored data (a name of its own,
//! a key bound to its address) are kept by `db/ai/connection/`.
//!
//! ```text
//! list()         GET     /connections
//! get()          GET     /connections/{id}
//! create()       POST    /connections
//! update()       PUT     /connections/{id}
//! delete()       DELETE  /connections/{id}
//! active()       GET     /connections/active
//! activate()     PUT     /connections/active
//! set_key()      PUT     /connections/{id}/key
//! delete_key()   DELETE  /connections/{id}/key
//! ```
//!
//! **A key is never given back**: a stored connection says only whether
//! it has one (`StoredConnection::has_key`).

use std::fmt;

use crate::db::ai::connection as db_connection;
use crate::db::ai::connection::rows::SaveError;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::{Connection, ConnectionError, SavedConnection, StoredConnection};
use crate::model::api_key::ApiKey;

/// Why a connection was not saved.
#[derive(Debug)]
pub enum ConnectionSaveError {
    /// A field breaks a rule; the page marks the field.
    Invalid(ConnectionError),
    /// Another connection has this name.
    NameTaken,
    /// The connection to change does not exist.
    NoSuchConnection,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for ConnectionSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionSaveError::Invalid(error) => write!(formatter, "{error}"),
            ConnectionSaveError::NameTaken => formatter.write_str("another connection has this name"),
            ConnectionSaveError::NoSuchConnection => formatter.write_str("the connection does not exist"),
            ConnectionSaveError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ConnectionSaveError {}

impl From<SaveError> for ConnectionSaveError {
    fn from(error: SaveError) -> ConnectionSaveError {
        match error {
            SaveError::NameTaken => ConnectionSaveError::NameTaken,
            SaveError::NoSuchConnection => ConnectionSaveError::NoSuchConnection,
            SaveError::Db(error) => ConnectionSaveError::Db(error),
        }
    }
}

/// The connection in its normal form, when it keeps the rules.
fn checked(connection: &Connection) -> Result<Connection, ConnectionSaveError> {
    let connection = connection.normalized();

    connection.check().map_err(ConnectionSaveError::Invalid)?;

    Ok(connection)
}

/// Every connection, by name.
pub fn list(db: &UserDb) -> Result<Vec<StoredConnection>, DbError> {
    db_connection::read::list(db)
}

/// One connection; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<StoredConnection>, DbError> {
    db_connection::read::get(db, id)
}

/// Stores a new connection; the first one becomes the active one.
pub fn create(db: &UserDb, connection: &Connection) -> Result<SavedConnection, ConnectionSaveError> {
    Ok(db_connection::create::connection(db, &checked(connection)?)?)
}

/// Changes connection `id`; its key is removed when its address
/// changed (`SavedConnection::key_removed`).
pub fn update(db: &UserDb, id: i64, connection: &Connection) -> Result<SavedConnection, ConnectionSaveError> {
    Ok(db_connection::update::connection(db, id, &checked(connection)?)?)
}

/// Deletes a connection with its key; returns whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    db_connection::delete::connection(db, id)
}

/// The active connection, if one is chosen.
pub fn active(db: &UserDb) -> Result<Option<i64>, DbError> {
    db_connection::read::active(db)
}

/// Makes a connection the active one; returns whether it exists.
pub fn activate(db: &UserDb, id: i64) -> Result<bool, DbError> {
    db_connection::update::active(db, id)
}

/// Stores the API key of a connection, bound to its address; returns
/// whether the connection exists.
pub fn set_key(db: &UserDb, id: i64, key: &ApiKey) -> Result<bool, DbError> {
    db_connection::update::api_key(db, id, key)
}

/// Removes the API key of a connection.
pub fn delete_key(db: &UserDb, id: i64) -> Result<(), DbError> {
    db_connection::delete::api_key(db, id)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "connections_tests.rs"]
mod tests;
