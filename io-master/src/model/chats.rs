//! The chats of an evaluator with an AI model, as data: a chat in the
//! list, a message, and how a reply ended. The work on them is in
//! `ai/chat/`; they are kept by `db/ai/memory/`.
//!
//! ```text
//! complete   the model finished
//! cut        the reply reached the largest length and was cut
//! stopped    the evaluator stopped it; the text up to then is kept
//! refused    the model declined; any partial text is dropped (it
//!            may be the beginning of what was declined), and only
//!            the fact is kept
//! failed     the request failed; the explanation is kept, and the
//!            text written before the failure
//! ```

use crate::model::ai::Speaker;

/// A chat in the list of chats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSummary {
    pub id: i64,
    pub title: String,
    /// Local time it was started: "2026-10-14 15:04".
    pub created: String,
}

/// How a reply of the model ended (see the top of this file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    Complete,
    Cut,
    Stopped,
    Refused,
    Failed,
}

impl Ending {
    /// The name kept in the database, also used by the chat page.
    pub fn name(&self) -> &'static str {
        match self {
            Ending::Complete => "complete",
            Ending::Cut => "cut",
            Ending::Stopped => "stopped",
            Ending::Refused => "refused",
            Ending::Failed => "failed",
        }
    }

    /// The ending with the name `name`; `None` for a name the
    /// program never writes.
    pub fn from_name(name: &str) -> Option<Ending> {
        match name {
            "complete" => Some(Ending::Complete),
            "cut" => Some(Ending::Cut),
            "stopped" => Some(Ending::Stopped),
            "refused" => Some(Ending::Refused),
            "failed" => Some(Ending::Failed),
            _ => None,
        }
    }
}

/// A message of a chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub speaker: Speaker,
    pub text: String,
    /// Local time it was written: "2026-10-14 15:04".
    pub created: String,
    /// For a reply: the model that wrote it.
    pub model: Option<String>,
    /// For a reply: how it ended. `Complete` for the messages of
    /// the evaluator.
    pub ending: Ending,
    /// For a failed reply: the explanation.
    pub note: Option<String>,
}
