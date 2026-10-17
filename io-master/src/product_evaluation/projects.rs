//! The evaluation projects of an evaluator, kept in their encrypted
//! database (the SQL is in `db/evaluations/`).
//!
//! ```text
//! create()   the frozen copy, written in one transaction with the
//!            first entry of the journal (used by import.rs)
//! list(), get(), data()   reading
//! tile()     one tile of an evaluation (used by results.rs)
//! delete()   the whole evaluation with its results and journal
//! ```
//!
//! The structure of an evaluation is never changed after `create`:
//! there is no function for it.

use crate::db::error::DbError;
use crate::db::evaluations::create::{self as store, Copy, Step};
use crate::db::evaluations::{delete as remove, read};
use crate::db::user::database::UserDb;
use crate::evaluation::scheme::Tile;

use super::model::{Actor, Evaluation, EvaluationData, EvaluationError, EvaluationFields, JournalAction, Origin};

/// Writes a new evaluation with its frozen structure and the entry
/// `detail` of its import in the journal; returns its number. The
/// fields are checked here; the structure is checked by `import.rs`.
/// On an error nothing of it is left.
pub fn create(
    db: &UserDb,
    fields: &EvaluationFields,
    origin: &Origin,
    copy: &Copy<'_>,
    detail: &str,
) -> Result<i64, EvaluationError> {
    fields.check()?;

    if read::name_taken(db, &fields.name)? {
        return Err(EvaluationError::NameTaken);
    }

    let step = Step {
        actor: &Actor::Evaluator,
        action: JournalAction::Imported,
        tile: None,
        field: None,
        detail,
    };

    Ok(store::evaluation(db, fields, origin, copy, &step)?)
}

/// Every evaluation, by name.
pub fn list(db: &UserDb) -> Result<Vec<Evaluation>, DbError> {
    read::evaluations(db)
}

/// One evaluation, without its structure; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Evaluation>, DbError> {
    read::evaluation(db, id)
}

/// An evaluation with its frozen structure; `None` when there is no such.
pub fn data(db: &UserDb, id: i64) -> Result<Option<EvaluationData>, DbError> {
    let Some(evaluation) = get(db, id)? else {
        return Ok(None);
    };

    Ok(Some(EvaluationData {
        evaluation,
        documents: read::documents(db, id)?,
        schemes: read::schemes(db, id)?,
        tiles: read::tiles(db, id)?,
        links: read::links(db, id)?,
    }))
}

/// One tile of an evaluation with its sources, and the evaluation it
/// belongs to; `None` when there is no such.
pub fn tile(db: &UserDb, id: i64) -> Result<Option<(i64, Tile)>, DbError> {
    read::tile(db, id)
}

/// Deletes an evaluation with its structure, results, proposals and
/// journal (overwritten in the file: `db/evaluations/delete.rs`);
/// returns whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    remove::evaluation(db, id)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
