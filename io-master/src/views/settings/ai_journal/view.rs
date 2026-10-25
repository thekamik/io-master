//! The data of the journal of the AI, as the page gets it (step 58 of
//! docs/todo.md): a page of its entries, and one entry whole. The journal
//! is only read here; it is written by `ai/api/` (P3).

use serde::Serialize;

use crate::db::ai::journal::read::PAGE_SIZE;
use crate::db::ai::journal::rows::{Entry, EntrySummary};

/// An entry as a row of the list.
#[derive(Debug, Serialize)]
pub struct JournalRowView {
    pub id: i64,
    /// Seconds since 1970 (UTC); the page shows them in its own time.
    pub time: i64,
    pub task: String,
    pub role: String,
    pub connection: String,
    pub model: String,
    pub ending: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// A page of the list, how many entries the filter keeps, and the tasks to
/// filter by.
#[derive(Debug, Serialize)]
pub struct JournalPageView {
    pub rows: Vec<JournalRowView>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub tasks: Vec<String>,
}

/// An entry whole: its row, the request as it was sent (indented when it
/// is JSON), the answer and why it failed.
#[derive(Debug, Serialize)]
pub struct JournalEntryView {
    pub row: JournalRowView,
    pub request: String,
    pub answer: String,
    pub note: Option<String>,
}

/// The row of `summary`.
fn row(summary: EntrySummary) -> JournalRowView {
    JournalRowView {
        id: summary.id,
        time: summary.time,
        task: summary.task,
        role: summary.role,
        connection: summary.connection,
        model: summary.model,
        ending: summary.ending,
        input_tokens: summary.input_tokens,
        output_tokens: summary.output_tokens,
    }
}

/// Page `page` of the list.
///
/// Called by: `ai_journal_get` (`settings.rs`).
pub fn page(summaries: Vec<EntrySummary>, total: usize, page: usize, tasks: Vec<String>) -> JournalPageView {
    let mut rows = Vec::with_capacity(summaries.len());

    for summary in summaries {
        rows.push(row(summary));
    }

    JournalPageView { rows, total, page, page_size: PAGE_SIZE, tasks }
}

/// `entry` whole, its request indented when it reads as JSON (as it is
/// kept otherwise).
///
/// Called by: `ai_journal_entry` (`settings.rs`).
pub fn entry(entry: Entry) -> JournalEntryView {
    let request = match serde_json::from_str::<serde_json::Value>(&entry.request) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or(entry.request),
        Err(_) => entry.request,
    };
    let summary = EntrySummary {
        id: entry.id,
        time: entry.time,
        task: entry.task,
        role: entry.role,
        connection: entry.connection,
        model: entry.model,
        ending: entry.ending,
        input_tokens: entry.input_tokens,
        output_tokens: entry.output_tokens,
    };

    JournalEntryView { row: row(summary), request, answer: entry.answer, note: entry.note }
}
