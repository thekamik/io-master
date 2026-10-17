//! Removing a document of a standard.

use crate::db::standards::{delete as remove, read};
use crate::db::user::database::UserDb;

use crate::standards::error::DocumentError;

/// Removes a document; refused while a test project or an
/// evaluation project uses it.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DocumentError> {
    if read::in_use(db, id)? {
        return Err(DocumentError::InUse);
    }

    // The licensed text is overwritten in the file (db/standards/delete.rs).
    Ok(remove::document(db, id)?)
}
