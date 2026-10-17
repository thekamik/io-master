//! Reading the system database: the installation and the accounts.

use rusqlite::OptionalExtension;

use crate::db::error::{Corrupt, DbError};
use crate::model::installation::{ID_LENGTH, Installation, InstallationId, Role};

use super::database::SystemDb;
use super::rows::{UserRecord, user_from_row};

/// The record of the installation.
pub fn installation(db: &SystemDb) -> Result<Installation, DbError> {
    let (id, role): (Vec<u8>, String) =
        db.connection().query_row("SELECT id, role FROM installation", [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let id: InstallationId = id.try_into().map_err(|id: Vec<u8>| {
        DbError::Corrupt(Corrupt {
            table: "installation",
            column: "id",
            value: format!("{} bytes, not {ID_LENGTH}", id.len()),
        })
    })?;

    let role = Role::from_name(&role)
        .ok_or_else(|| DbError::Corrupt(Corrupt { table: "installation", column: "role", value: role.clone() }))?;

    Ok(Installation { id, role })
}

/// The names of all accounts, the oldest first.
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
