//! Adding an entry to the journal of the AI (the only way an entry
//! comes into being; none is ever changed or removed).

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::rows::Entry;

/// Adds `entry` (its `id` and `time` are ignored: the database gives
/// them); returns its number.
pub fn entry(db: &UserDb, entry: &Entry) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO ai_journal (task, role, connection, model, request, answer, ending, note, input_tokens, \
         output_tokens) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            entry.task,
            entry.role,
            entry.connection,
            entry.model,
            entry.request,
            entry.answer,
            entry.ending,
            entry.note,
            entry.input_tokens as i64,
            entry.output_tokens as i64
        ],
    )?;

    Ok(db.connection().last_insert_rowid())
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, the triggers).
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
