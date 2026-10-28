//! An entry of the journal of the AI and its row.

use rusqlite::Row;

use crate::db_old::error::{Corrupt, DbError};

/// One question to the AI and its answer, as the journal keeps them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The number; 0 for an entry not written yet.
    pub id: i64,
    /// Seconds since 1970 (UTC); 0 for an entry not written yet (the
    /// database sets the time).
    pub time: i64,
    /// The task, by name (`ai/api/tasks.rs`).
    pub task: String,
    /// The role, by name (`ai/api/roles.rs`); empty for a check of a
    /// connection, which has no role.
    pub role: String,
    /// The name of the connection.
    pub connection: String,
    /// The model that answered, else the model of the connection.
    pub model: String,
    /// The body of the request as it was sent (JSON); never a header.
    pub request: String,
    /// The text of the answer, complete or not.
    pub answer: String,
    /// How it ended: complete, cut, refused, stopped or failed.
    pub ending: String,
    /// Why it failed, when it did.
    pub note: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// An entry as a row of the list of the journal (step 58): all but the
/// request and the answer, which are read when the entry is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySummary {
    pub id: i64,
    pub time: i64,
    pub task: String,
    pub role: String,
    pub connection: String,
    pub model: String,
    pub ending: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// The tokens of column `column`; a negative number, which the program
/// never writes, is corrupt.
fn tokens(row: &Row<'_>, column: &'static str) -> Result<u64, DbError> {
    let value: i64 = row.get(column)?;

    u64::try_from(value)
        .map_err(|_| DbError::Corrupt(Corrupt { table: "ai_journal", column, value: value.to_string() }))
}

/// A whole row of `ai_journal`.
///
/// Called by: `read::entry`, `read::last` (the tests).
pub(super) fn entry(row: &Row<'_>) -> Result<Entry, DbError> {
    Ok(Entry {
        id: row.get("id")?,
        time: row.get("time")?,
        task: row.get("task")?,
        role: row.get("role")?,
        connection: row.get("connection")?,
        model: row.get("model")?,
        request: row.get("request")?,
        answer: row.get("answer")?,
        ending: row.get("ending")?,
        note: row.get("note")?,
        input_tokens: tokens(row, "input_tokens")?,
        output_tokens: tokens(row, "output_tokens")?,
    })
}

/// A row of `ai_journal` without its request and answer.
///
/// Called by: `read::summaries`.
pub(super) fn summary(row: &Row<'_>) -> Result<EntrySummary, DbError> {
    Ok(EntrySummary {
        id: row.get("id")?,
        time: row.get("time")?,
        task: row.get("task")?,
        role: row.get("role")?,
        connection: row.get("connection")?,
        model: row.get("model")?,
        ending: row.get("ending")?,
        input_tokens: tokens(row, "input_tokens")?,
        output_tokens: tokens(row, "output_tokens")?,
    })
}
