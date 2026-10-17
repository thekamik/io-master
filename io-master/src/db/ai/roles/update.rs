//! Giving a role of the AI a connection of its own, or taking it away.

use rusqlite::params;

use crate::db::ai::connection::read as connections;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::ai::Role;

/// Gives `role` the connection `id`, or none (`None`: the role uses the
/// active connection). Returns `false` when the connection does not
/// exist; nothing is changed then.
pub fn connection(db: &UserDb, role: Role, id: Option<i64>) -> Result<bool, DbError> {
    if let Some(id) = id
        && connections::get(db, id)?.is_none()
    {
        return Ok(false);
    }

    db.connection().execute(
        "INSERT INTO ai_roles (role, connection_id) VALUES (?1, ?2) \
         ON CONFLICT (role) DO UPDATE SET connection_id = excluded.connection_id",
        params![role.name(), id],
    )?;

    Ok(true)
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (read, update).
#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
