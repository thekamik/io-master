//! Deleting from the evaluation support.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes quote `id`; returns whether it existed.
pub fn quote(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM support_quotes WHERE id = :id", named_params! { ":id": id })? > 0)
}

/// Deletes the quotes of the objects `object_ids` of document
/// `document_uuid` (the objects were removed from it).
pub fn quotes_of(db: &UserDb, document_uuid: &str, object_ids: &[String]) -> Result<(), DbError> {
    for object in object_ids {
        db.connection().execute(
            "DELETE FROM support_quotes WHERE document_uuid = :document AND object_id = :object",
            named_params! { ":document": document_uuid, ":object": object },
        )?;
    }

    Ok(())
}

/// Deletes proposal `id` (accepted into the document, or rejected);
/// returns whether it existed.
pub fn proposal(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM support_proposals WHERE id = :id", named_params! { ":id": id })? > 0)
}

/// Deletes everything of the evaluation support of document
/// `document_uuid` (the document was deleted).
pub fn of_document(db: &UserDb, document_uuid: &str) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM support_quotes WHERE document_uuid = :document",
        named_params! { ":document": document_uuid },
    )?;
    transaction.execute(
        "DELETE FROM support_proposals WHERE document_uuid = :document",
        named_params! { ":document": document_uuid },
    )?;
    transaction.commit()?;

    Ok(())
}
