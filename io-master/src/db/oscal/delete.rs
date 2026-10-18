//! Deleting a document of OSCAL. Its text is overwritten in the file,
//! not only marked free: a plan or results may hold what a manufacturer
//! gave under a non-disclosure agreement.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes document `uuid`; returns whether it existed.
pub fn document(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute_batch("PRAGMA secure_delete = ON")?;
    let deleted =
        transaction.execute("DELETE FROM oscal_documents WHERE uuid = :uuid", named_params! { ":uuid": uuid })? > 0;
    transaction.commit()?;

    Ok(deleted)
}
