//! The rows of the chats and their messages, and the Rust types they
//! become (`model/chats.rs`). Columns are read by their names; a value
//! the program never writes is an error (`DbError::Corrupt`).

use rusqlite::Row;

use crate::db::error::{corrupt, named};
use crate::model_old::ai::Speaker;
use crate::model_old::chats::{ChatMessage, ChatSummary, Ending};

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

/// A chat from a row of `chats` with `local_created`, the local time
/// it was started ([`local_time`]).
pub(super) fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<ChatSummary> {
    Ok(ChatSummary { id: row.get("id")?, title: row.get("title")?, created: row.get("local_created")? })
}

/// A message from a row of `messages` with `local_created`. A message
/// of the evaluator has no ending (NULL): it is complete.
pub(super) fn message_from_row(row: &Row<'_>) -> rusqlite::Result<ChatMessage> {
    let speaker: String = row.get("speaker")?;
    let ending: Option<String> = row.get("ending")?;

    Ok(ChatMessage {
        speaker: match speaker.as_str() {
            "evaluator" => Speaker::Evaluator,
            "model" => Speaker::Model,
            _ => return Err(corrupt("messages", "speaker", &speaker)),
        },
        text: row.get("text")?,
        created: row.get("local_created")?,
        model: row.get("model")?,
        ending: match ending {
            Some(ending) => named("messages", "ending", &ending, Ending::from_name)?,
            None => Ending::Complete,
        },
        note: row.get("note")?,
    })
}
