//! Changing what the user told about a document of a standard.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model_old::standards::DocumentFields;

/// Changes the title, part and version of a document; returns whether
/// it exists.
pub fn document(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "UPDATE standard_documents SET title = :title, part = :part, version = :version WHERE id = :id",
        named_params! { ":title": fields.title, ":part": fields.part, ":version": fields.version, ":id": id },
    )? > 0)
}
