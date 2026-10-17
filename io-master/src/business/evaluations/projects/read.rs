//! Reading evaluation projects as they are stored: the list, one
//! evaluation, and its frozen structure (to open it, `workspace.rs`).

use crate::db::error::DbError;
use crate::db::evaluations::read;
use crate::db::user::database::UserDb;

use crate::model::evaluations::{Evaluation, EvaluationData};

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

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, delete) and the workspace.
#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
