//! The rows of the journal of an evaluation and the entries they
//! become (`model/evaluations.rs`). A value the program never writes is
//! an error (`DbError::Corrupt`): a journal is evidence, and is shown
//! as it is or not at all.

use rusqlite::Row;

use crate::db::error::{corrupt, named};
use crate::model::evaluations::{Actor, JournalAction, JournalEntry, ResultField};

const TABLE: &str = "evaluation_journal";

/// The name of an actor in the journal, and the model it names.
pub(super) fn actor_columns(actor: &Actor) -> (&'static str, Option<&str>) {
    match actor {
        Actor::Evaluator => ("evaluator", None),
        Actor::Model(model) => ("model", Some(model.as_str())),
    }
}

/// An entry from a row of `evaluation_journal`.
pub(super) fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<JournalEntry> {
    let actor: String = row.get("actor")?;
    let model: Option<String> = row.get("model")?;
    let action: String = row.get("action")?;
    let field: Option<String> = row.get("field")?;

    Ok(JournalEntry {
        id: row.get("id")?,
        time: row.get("time")?,
        actor: match (actor.as_str(), model) {
            ("evaluator", None) => Actor::Evaluator,
            ("model", Some(model)) => Actor::Model(model),
            _ => return Err(corrupt(TABLE, "actor", &actor)),
        },
        action: named(TABLE, "action", &action, JournalAction::from_name)?,
        tile: row.get("tile_id")?,
        field: field.map(|field| named(TABLE, "field", &field, ResultField::from_name)).transpose()?,
        detail: row.get("detail")?,
    })
}
