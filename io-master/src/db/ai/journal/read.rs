//! Reading the journal of the AI.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::rows::{COLUMNS, Entry, entry_from_row};

/// The last `count` entries, the newest first.
pub fn last(db: &UserDb, count: usize) -> Result<Vec<Entry>, DbError> {
    let mut statement =
        db.connection().prepare(&format!("SELECT {COLUMNS} FROM ai_journal ORDER BY id DESC LIMIT ?1"))?;

    Ok(statement.query_map([count as i64], entry_from_row)?.collect::<Result<Vec<_>, _>>()?)
}
