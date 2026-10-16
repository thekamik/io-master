//! The chats as the page shows them, and the events of a reply
//! while it is written.
//!
//! A reply reaches the page through a channel of Tauri, as these
//! events:
//!
//! ```text
//! {"event": "chat", "id": 7, "title": "..."}    first: the chat
//! {"event": "text", "text": "a piece"}          many
//! {"event": "end", "ending": "complete", "notice": null}   last
//! ```

use serde::Serialize;

use crate::ai::request::Speaker;
use crate::ai::chats::{ChatMessage, ChatSummary, Ending};

/// A chat in the list.
#[derive(Debug, Serialize)]
pub struct ChatSummaryView {
    pub id: i64,
    pub title: String,
    /// Local time it was started: "2026-10-14 15:04".
    pub created: String,
}

impl From<ChatSummary> for ChatSummaryView {
    fn from(chat: ChatSummary) -> ChatSummaryView {
        ChatSummaryView {
            id: chat.id,
            title: chat.title,
            created: chat.created,
        }
    }
}

/// A message of a chat.
#[derive(Debug, Serialize)]
pub struct MessageView {
    /// `"evaluator"` or `"model"`.
    pub speaker: &'static str,
    /// Who wrote it, as the page shows it: "You", or the model.
    pub who: String,
    pub text: String,
    pub created: String,
    /// The line under a reply that did not end normally.
    pub notice: Option<String>,
}

impl From<ChatMessage> for MessageView {
    fn from(message: ChatMessage) -> MessageView {
        let (speaker, who) = match message.speaker {
            Speaker::Evaluator => ("evaluator", "You".to_owned()),
            Speaker::Model => ("model", message.model.clone().unwrap_or_else(|| "Model".to_owned())),
        };

        MessageView {
            speaker,
            who,
            notice: ending_notice(&message.ending, message.note.as_deref()),
            text: message.text,
            created: message.created,
        }
    }
}

/// An open chat.
#[derive(Debug, Serialize)]
pub struct ChatView {
    pub id: i64,
    pub title: String,
    pub messages: Vec<MessageView>,
}

/// An event of a reply being written.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ReplyEvent {
    /// The chat the message went to (a new one for a first
    /// message).
    Chat { id: i64, title: String },
    /// A piece of the reply.
    Text { text: String },
    /// The reply is over and stored.
    End {
        /// `"complete"`, `"cut"`, `"stopped"`, `"refused"` or
        /// `"failed"`.
        ending: &'static str,
        notice: Option<String>,
    },
}

/// What the page says under a reply that did not end normally;
/// `None` for a complete reply.
pub fn ending_notice(ending: &Ending, note: Option<&str>) -> Option<String> {
    match ending {
        Ending::Complete => None,
        Ending::Cut => Some("The reply reached its largest length and was cut.".to_owned()),
        Ending::Stopped => Some("Stopped.".to_owned()),
        Ending::Refused => Some("The model declined to answer this request.".to_owned()),
        Ending::Failed => Some(format!("The reply failed: {}.", note.unwrap_or("no explanation"))),
    }
}
