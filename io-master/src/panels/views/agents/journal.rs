//! The journal of an agent, as the page reads it: one page of its entries,
//! the newest first, and one entry whole (its request and its answer). The
//! journal is only read here: entries are written by `ai/ask.rs` alone.
//!
//! ```text
//! AgentJournal.svelte ──agent_journal_list(agent, before)─► read_journal_entries ──► [JournalRowView] (50 at most)
//!                     ──agent_journal_get(id)─────────────► read_journal_entry   ──► JournalEntryView
//! ```

use serde::Serialize;
use tauri::State;

use crate::db::ai::journal::{read_journal_entries, read_journal_entry};
use crate::model::ai::journal::{Ending, JournalEntry};
use crate::panels::views::error::ViewError;
use crate::panels::views::opened::database;
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

/// The name of an ending on the page.
///
/// Called by: [`row_of`].
fn ending_name(ending: Ending) -> &'static str {
    match ending {
        Ending::Complete => "complete",
        Ending::Cut => "cut",
        Ending::Stopped => "stopped",
        Ending::Refused => "refused",
        Ending::Failed => "failed",
    }
}

/// The row of the entry `entry`.
///
/// Called by: [`agent_journal_list`], [`agent_journal_get`].
fn row_of(entry: &JournalEntry) -> JournalRowView {
    JournalRowView {
        id: entry.id,
        time: entry.time,
        integration: entry.integration.clone(),
        model: entry.model.clone(),
        work_unit: entry.work_unit,
        ending: ending_name(entry.ending),
        input_tokens: entry.input_tokens,
        output_tokens: entry.output_tokens,
    }
}

/// One page of the journal of the agent `agent`, the newest first: its
/// entries older than `before` (the newest with `None`).
///
/// Called by: `AgentJournal.svelte`, when an agent is opened and for "Older".
/// Calls: `read_journal_entries`, [`row_of`].
#[tauri::command]
pub fn agent_journal_list(
    session: State<'_, Session>,
    agent: i64,
    before: Option<i64>,
) -> Result<Vec<JournalRowView>, ViewError> {
    let opened = session.lock()?;
    let entries = read_journal_entries(database(&opened)?, agent, before)?;
    let mut rows = Vec::with_capacity(entries.len());

    for entry in &entries {
        rows.push(row_of(entry));
    }

    Ok(rows)
}

/// The entry `id` whole.
///
/// Called by: `AgentJournal.svelte`, when an entry is opened.
/// Calls: `read_journal_entry`, [`row_of`].
#[tauri::command]
pub fn agent_journal_get(session: State<'_, Session>, id: i64) -> Result<JournalEntryView, ViewError> {
    let opened = session.lock()?;
    let entry = read_journal_entry(database(&opened)?, id)?;

    Ok(JournalEntryView { row: row_of(&entry), request: entry.request, answer: entry.answer })
}
