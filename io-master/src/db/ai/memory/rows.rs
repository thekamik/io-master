//! The rows of the chats and their messages, and the Rust types they
//! become (`ai/chat/chats.rs`).

use rusqlite::Row;

use crate::ai::chat::chats::{ChatMessage, ChatSummary, Ending};
use crate::ai::api::completions::Speaker;

/// The current time in SQL, as seconds since 1970.
pub(super) const NOW: &str = "CAST(strftime('%s', 'now') AS INTEGER)";

/// The local time of a column of seconds, as "2026-10-14 15:04".
pub(super) fn local_time(column: &str) -> String {
    format!("strftime('%Y-%m-%d %H:%M', {column}, 'unixepoch', 'localtime')")
}

/// The name of a speaker in the database.
pub(super) fn speaker_name(speaker: Speaker) -> &'static str {
    match speaker {
        Speaker::Evaluator => "evaluator",
        Speaker::Model => "model",
    }
}

/// A chat from a row of `id, title, local time of created`.
pub(super) fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<ChatSummary> {
    Ok(ChatSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        created: row.get(2)?,
    })
}

/// A message from a row of `speaker, text, local time of created,
/// model, ending, note`.
pub(super) fn message_from_row(row: &Row<'_>) -> rusqlite::Result<ChatMessage> {
    let speaker: String = row.get(0)?;
    let ending: Option<String> = row.get(4)?;

    Ok(ChatMessage {
        speaker: if speaker == "model" { Speaker::Model } else { Speaker::Evaluator },
        text: row.get(1)?,
        created: row.get(2)?,
        model: row.get(3)?,
        ending: ending.as_deref().map(Ending::from_name).unwrap_or(Ending::Complete),
        note: row.get(5)?,
    })
}
