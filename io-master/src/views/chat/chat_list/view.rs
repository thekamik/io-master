//! The data of the list of chats, as the page gets it.

use serde::Serialize;

use crate::model_old::chats::ChatSummary;

/// A chat in the list.
#[derive(Debug, Serialize)]
pub struct ChatSummaryView {
    pub id: i64,
    pub title: String,
    /// Local time it was started: "2026-10-14 15:04".
    pub created: String,
}

impl ChatSummaryView {
    /// The views of `chats`, in their order.
    pub fn all(chats: Vec<ChatSummary>) -> Vec<ChatSummaryView> {
        let mut views = Vec::with_capacity(chats.len());

        for chat in chats {
            views.push(ChatSummaryView { id: chat.id, title: chat.title, created: chat.created });
        }

        views
    }
}
