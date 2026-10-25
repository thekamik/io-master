//! Deleting knowledge bases and their sources.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes base `id` with its sources and their passages, in one
/// transaction; gives whether it existed.
///
/// Called by: `knowledge_list_delete` (`views/knowledge/knowledge.rs`).
pub fn base(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM knowledge_passages WHERE source_id IN (SELECT id FROM knowledge_sources WHERE base_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM knowledge_sources WHERE base_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM knowledge_bases WHERE id = ?1", [id])?;
    transaction.commit()?;

    Ok(deleted > 0)
}

/// Deletes source `source` of base `base` with its passages, in one
/// transaction; gives whether it existed.
///
/// Called by: `knowledge_base_remove` (`views/knowledge/knowledge.rs`).
pub fn source(db: &UserDb, base: i64, source: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let deleted = transaction.execute(
        "DELETE FROM knowledge_sources WHERE id = :source AND base_id = :base",
        named_params! { ":source": source, ":base": base },
    )?;

    if deleted > 0 {
        transaction.execute("DELETE FROM knowledge_passages WHERE source_id = ?1", [source])?;
    }

    transaction.commit()?;

    Ok(deleted > 0)
}
