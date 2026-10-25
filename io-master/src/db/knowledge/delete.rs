//! Deleting knowledge bases, their sources, agents and their trials.

use rusqlite::{Transaction, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes base `id` with its sources and their passages, in one
/// transaction; the agents that drew on it draw on it no more. Gives
/// whether it existed.
///
/// Called by: `knowledge_list_delete` (`views/knowledge/knowledge.rs`).
pub fn base(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM knowledge_passages WHERE source_id IN (SELECT id FROM knowledge_sources WHERE base_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM knowledge_sources WHERE base_id = ?1", [id])?;
    transaction.execute("DELETE FROM knowledge_agent_bases WHERE base_id = ?1", [id])?;
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

/// Deletes agent `id` with its bases, its trial and its attachments, in
/// one transaction; gives whether it existed. The bases themselves stay.
///
/// Called by: `agent_list_delete` (`views/knowledge/knowledge.rs`).
/// Calls: [`trial_of`].
pub fn agent(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    trial_of(&transaction, id)?;
    transaction.execute("DELETE FROM knowledge_agent_bases WHERE agent_id = ?1", [id])?;
    transaction.execute("DELETE FROM knowledge_attachments WHERE agent_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM knowledge_agents WHERE id = ?1", [id])?;
    transaction.commit()?;

    Ok(deleted > 0)
}

/// Deletes the trial of agent `agent`, inside `transaction`.
///
/// Called by: [`agent`], `update::trial`.
pub(super) fn trial_of(transaction: &Transaction<'_>, agent: i64) -> Result<(), DbError> {
    transaction.execute("DELETE FROM knowledge_trial_quotes WHERE agent_id = ?1", [agent])?;
    transaction.execute("DELETE FROM knowledge_trial_passages WHERE agent_id = ?1", [agent])?;
    transaction.execute("DELETE FROM knowledge_trials WHERE agent_id = ?1", [agent])?;

    Ok(())
}
