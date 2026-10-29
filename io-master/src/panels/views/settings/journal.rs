//! The journal of the AI, as the page reads it: one page of entries, the
//! newest first, and one entry whole (its request and its answer). The
//! journal is only read here: entries are written by `ai/ask.rs` alone.
//!
//! ```text
//! Journal.svelte ──journal_list(before)─► read_journal_entries ──► [JournalRowView]   (50 at most)
//!                ──journal_get(id)──────► read_journal_entry   ──► JournalEntryView
//! ```

use serde::Serialize;
use tauri::State;

use crate::db::ai::agent::read_agent;
use crate::db::ai::journal::{read_journal_entries, read_journal_entry};
use crate::db::database::Database;
use crate::model::ai::journal::JournalEntry;
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
use crate::panels::views::settings::integrations::ending_name;
use crate::session::state::Session;

/// An entry of the journal, as its list shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRowView {
    id: i64,
    /// Seconds since 1970 (UTC).
    time: i64,
    integration: String,
    model: String,
    /// The name of the agent; empty without one.
    agent: String,
    work_unit: Option<i64>,
    /// "complete", "cut", "stopped", "refused", "failed".
    ending: &'static str,
    input_tokens: u32,
    output_tokens: u32,
}

/// An entry of the journal whole.
#[derive(Serialize)]
pub struct JournalEntryView {
    row: JournalRowView,
    /// The request as it was sent (JSON; the key is never in it).
    request: String,
    /// The answer as it came back.
    answer: String,
}

/// The row of the entry `entry`, its agent named.
///
/// Called by: [`journal_list`], [`journal_get`].
/// Calls: `read_agent`.
fn row_of(db: &Database, entry: &JournalEntry) -> Result<JournalRowView, ViewError> {
    let agent = match entry.agent {
        Some(id) => read_agent(db, id)?.name,
        None => String::new(),
    };

    Ok(JournalRowView {
        id: entry.id,
        time: entry.time,
        integration: entry.integration.clone(),
        model: entry.model.clone(),
        agent,
        work_unit: entry.work_unit,
        ending: ending_name(entry.ending),
        input_tokens: entry.input_tokens,
        output_tokens: entry.output_tokens,
    })
}

/// One page of the journal, the newest first: the entries older than
/// `before` (the newest with `None`).
///
/// Called by: `Journal.svelte`, when the tab opens and for "Older".
/// Calls: `read_journal_entries`, [`row_of`].
#[tauri::command]
pub fn journal_list(session: State<'_, Session>, before: Option<i64>) -> Result<Vec<JournalRowView>, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let entries = read_journal_entries(db, before)?;
    let mut rows = Vec::with_capacity(entries.len());

    for entry in &entries {
        rows.push(row_of(db, entry)?);
    }

    Ok(rows)
}

/// The entry `id` whole.
///
/// Called by: `Journal.svelte`, when an entry is opened.
/// Calls: `read_journal_entry`, [`row_of`].
#[tauri::command]
pub fn journal_get(session: State<'_, Session>, id: i64) -> Result<JournalEntryView, ViewError> {
    let opened = session.lock()?;
    let db = database(&opened)?;
    let entry = read_journal_entry(db, id)?;

    Ok(JournalEntryView { row: row_of(db, &entry)?, request: entry.request, answer: entry.answer })
}
