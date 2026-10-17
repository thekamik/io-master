//! Reading the journal of an evaluation.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluations::JournalEntry;

use super::rows::entry_from_row;

/// The journal of an evaluation, the oldest entry first.
pub fn entries(db: &UserDb, evaluation: i64) -> Result<Vec<JournalEntry>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT * FROM evaluation_journal WHERE evaluation_id = :evaluation ORDER BY id")?;

    Ok(statement
        .query_map(named_params! { ":evaluation": evaluation }, entry_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}
