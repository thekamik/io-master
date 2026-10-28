//! A chat: the place where the user consults an agent, for example the
//! agent of one work unit, with its knowledge, about that work unit.
//! Kept in the tables `chats` and `chat_messages`. Every question of a
//! chat is also in the journal of the AI.

/// A chat with an agent (proposal, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Chat {
    /// The id of the chat in the database.
    pub id: i64,

    /// The title of the chat, for the list of chats.
    pub title: String,

    /// The agent consulted (`Agent`); `None`: the integration of the
    /// settings, without knowledge.
    pub agent: Option<i64>,

    /// The work unit the chat is about (`WorkUnit`), given to the agent
    /// with every question; `None` for a chat about nothing in
    /// particular.
    pub work_unit: Option<i64>,

    /// The messages, from the first.
    pub messages: Vec<Message>,
}

/// A message of a chat (proposal, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Message {
    /// Who wrote it.
    pub speaker: Speaker,

    /// The text of the message.
    pub text: String,

    /// When it was written: seconds since 1970 (UTC).
    pub time: i64,
}

/// Who wrote a message of a chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub enum Speaker {
    User,
    Ai,
}
