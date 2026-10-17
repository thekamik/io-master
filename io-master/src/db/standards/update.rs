//! Changing what the user told about a document of a standard.

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::standards::DocumentFields;

/// Changes the title, part and version of a document; returns whether
/// it exists.
pub fn document(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "UPDATE standard_documents SET title = ?1, part = ?2, version = ?3 WHERE id = ?4",
        params![fields.title, fields.part, fields.version, id],
    )? > 0)
}
