//! Deleting evaluations and proposals of the AI.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::journal::create::{JournalStep, entry};

/// Removes a proposal the evaluator rejected, with the entry `step`, in
/// one transaction; the result of its field is not touched.
pub fn proposal(db: &UserDb, evaluation: i64, id: i64, step: &JournalStep<'_>) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM evaluation_proposals WHERE id = :id", named_params! { ":id": id })?;
    entry(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(())
}

/// Deletes an evaluation with its structure, results, proposals and
/// journal, in one transaction; returns whether it existed. The results
/// may hold what the manufacturer gave under a non-disclosure
/// agreement, so they are overwritten in the file, not only marked
/// free.
pub fn evaluation(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let tiles = "(SELECT id FROM evaluation_tiles WHERE evaluation_id = :id)";

    transaction.execute_batch("PRAGMA secure_delete = ON")?;
    transaction
        .execute(&format!("DELETE FROM evaluation_results WHERE tile_id IN {tiles}"), named_params! { ":id": id })?;
    transaction
        .execute(&format!("DELETE FROM evaluation_proposals WHERE tile_id IN {tiles}"), named_params! { ":id": id })?;
    transaction.execute(
        &format!("DELETE FROM evaluation_tile_sources WHERE tile_id IN {tiles}"),
        named_params! { ":id": id },
    )?;
    transaction.execute("DELETE FROM evaluation_links WHERE evaluation_id = :id", named_params! { ":id": id })?;
    transaction.execute("DELETE FROM evaluation_tiles WHERE evaluation_id = :id", named_params! { ":id": id })?;
    transaction.execute("DELETE FROM evaluation_schemes WHERE evaluation_id = :id", named_params! { ":id": id })?;
    transaction.execute("DELETE FROM evaluation_documents WHERE evaluation_id = :id", named_params! { ":id": id })?;
    let deleted = transaction.execute("DELETE FROM evaluations WHERE id = :id", named_params! { ":id": id })? > 0;
    // Allowed by the database only now that the evaluation is gone.
    transaction.execute("DELETE FROM evaluation_journal WHERE evaluation_id = :id", named_params! { ":id": id })?;
    transaction.commit()?;

    Ok(deleted)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "delete_tests.rs"]
mod tests;
