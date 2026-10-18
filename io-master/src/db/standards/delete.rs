//! Removing a document of a standard. Whether a project still uses it
//! is checked by `business/standards/` (step 41: whether an OSCAL document
//! names it as a resource).

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Removes a document with the text of its pages, in one transaction;
/// returns whether it existed.
pub fn document(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    // Licensed text is overwritten in the file, not only marked free.
    transaction.execute_batch("PRAGMA secure_delete = ON")?;
    transaction.execute("DELETE FROM standard_pages WHERE document_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM standard_documents WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}
