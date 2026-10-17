//! The memory of a chat: what of its history goes to the model with
//! the next message.
//!
//! **Today: the whole history**, word for word, except the replies the
//! model refused, whose text is already dropped (`chats.rs`).
//!
//! The memory may be extended later (user, 2026-10-17): shortening the
//! history to the context of a model, summaries and other operations
//! on text. They are always controlled by parameters, and text that
//! must stay word for word is never touched: the quotes of a standard,
//! the fragments of a document (`ai/api/roles.rs`, `MemoryRule`).

use crate::ai::api::completions::Turn;

use super::chats::{ChatMessage, Ending};

/// The conversation as it is sent to the model: every message with
/// text, except refused replies, whose text is already dropped.
pub fn turns(messages: &[ChatMessage]) -> Vec<Turn> {
    messages
        .iter()
        .filter(|message| message.ending != Ending::Refused && !message.text.is_empty())
        .map(|message| Turn { speaker: message.speaker, text: message.text.clone() })
        .collect()
}
