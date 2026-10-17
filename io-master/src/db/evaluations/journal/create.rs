//! Adding an entry to the journal of an evaluation.
//!
//! **The journal is only added to:** no file of `db/evaluations/`
//! changes or removes an entry, and the database refuses it as well
//! (two triggers, `schema.rs`). An entry is written by the operation it
//! records, in the same transaction (`create.rs`, `update.rs`,
//! `delete.rs`), so there is never a change without its entry, nor an
//! entry of a change that did not happen.

use rusqlite::{Connection, named_params};

use crate::model::evaluations::{Actor, JournalAction, ResultField};

use super::rows::actor_columns;

/// One entry of the journal, to be written with the change it records.
#[derive(Debug, Clone)]
pub struct JournalStep<'a> {
    pub actor: &'a Actor,
    pub action: JournalAction,
    pub tile: Option<i64>,
    pub field: Option<ResultField>,
    pub detail: &'a str,
}

/// Adds `step` to the journal of `evaluation`. `connection` is the
/// open transaction of the change the entry records (a `Transaction`
/// dereferences to a `Connection`).
pub(in crate::db::evaluations) fn entry(
    connection: &Connection,
    evaluation: i64,
    step: &JournalStep<'_>,
) -> rusqlite::Result<()> {
    let (actor, model) = actor_columns(step.actor);

    connection.execute(
        "INSERT INTO evaluation_journal (evaluation_id, actor, model, action, tile_id, field, detail) \
         VALUES (:evaluation, :actor, :model, :action, :tile, :field, :detail)",
        named_params! {
            ":evaluation": evaluation,
            ":actor": actor,
            ":model": model,
            ":action": step.action.name(),
            ":tile": step.tile,
            ":field": step.field.map(ResultField::name),
            ":detail": step.detail,
        },
    )?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, the triggers).
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
