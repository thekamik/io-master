//! An entry of the journal of the AI and its row.

use rusqlite::Row;

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

/// The columns of an entry, in the order of [`entry_from_row`].
pub(super) const COLUMNS: &str =
    "id, time, task, role, connection, model, request, answer, ending, note, input_tokens, output_tokens";

pub(super) fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        time: row.get(1)?,
        task: row.get(2)?,
        role: row.get(3)?,
        connection: row.get(4)?,
        model: row.get(5)?,
        request: row.get(6)?,
        answer: row.get(7)?,
        ending: row.get(8)?,
        note: row.get(9)?,
        input_tokens: row.get::<_, i64>(10)? as u64,
        output_tokens: row.get::<_, i64>(11)? as u64,
    })
}
