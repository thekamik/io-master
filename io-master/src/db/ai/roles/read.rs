//! Reading which connection a role of the AI uses.

use rusqlite::OptionalExtension;

use crate::ai::api::roles::Role;
use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// The connection given to `role`; `None` when it has none (it uses
/// the active connection).
pub fn connection(db: &UserDb, role: Role) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT connection_id FROM ai_roles WHERE role = ?1", [role.name()], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .optional()?
        .flatten())
}
