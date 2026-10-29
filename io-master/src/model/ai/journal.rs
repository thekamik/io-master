//! The journal of the AI: every question an agent asked an AI service and
//! the answer that came back, so that every result of the AI can be traced
//! to what it rests on. Each agent has its own journal: the entries that
//! name it. A question without an agent (the check of an integration) is
//! not kept (user, 2026-10-29): it is no part of an evaluation. Kept in the table `ai_journal`, only ever
//! added to: the database refuses to change or remove an entry.
//!
//! The integration and the model are kept by their names, as text: the
//! entry stays as it was when they are renamed or removed. The agent is
//! named by its id (user, 2026-10-29): agents are never removed, so the id
//! always finds it.

/// An entry of the journal of the AI (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// The id of the entry in the database.
    pub id: i64,

    /// When the question was asked: seconds since 1970 (UTC).
    pub time: i64,

    /// The name of the integration asked.
    pub integration: String,

    /// The model that answered, as the service named it.
    pub model: String,

    /// The agent that asked (`Agent`); every entry has one.
    pub agent: i64,

    /// The work unit the question was asked for; `None` for a question
    /// outside an evaluation.
    pub work_unit: Option<i64>,

    /// The request as it was sent (the API key is never in it).
    pub request: String,

    /// The answer as it came back.
    pub answer: String,

    /// How the answer ended.
    pub ending: Ending,

    /// The tokens of the question and of the answer, as the service
    /// counted them.
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// How an answer of the AI ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The model finished its answer.
    Complete,
    /// The answer reached its largest length and was cut.
    Cut,
    /// The user stopped it.
    Stopped,
    /// The model declined to answer.
    Refused,
    /// The request failed.
    Failed,
}
