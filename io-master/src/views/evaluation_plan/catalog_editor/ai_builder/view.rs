//! The data of the AI of the editor, as the page gets and sends it, and
//! the events of its work:
//!
//! ```text
//! {"event": "part", "index": 1, "count": 3, "document": "...", "first_page": 1, "last_page": 9}
//! {"event": "writing", "characters": 1830}                the model is answering
//! {"event": "proposed", "added": 7, "dropped": ["..."]}    what a part gave
//! {"event": "failed", "message": "..."}                   this part gave nothing
//! {"event": "end", "stopped": false}                      last
//! ```

use serde::{Deserialize, Serialize};

use crate::model::standards::Document;

/// Pages of a document to read, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRange {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
}

/// A document the AI can read.
#[derive(Debug, Serialize)]
pub struct SourceView {
    pub id: i64,
    /// Its title and part.
    pub title: String,
    pub page_count: u32,
}

/// The views of `documents`, in their order.
pub fn sources(documents: &[Document]) -> Vec<SourceView> {
    let mut views = Vec::with_capacity(documents.len());

    for document in documents {
        views.push(SourceView { id: document.id, title: document.title_and_part(), page_count: document.page_count });
    }

    views
}

/// An event of a work of the AI.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum BuildEvent {
    /// A part starts (counted from 1).
    Part { index: usize, count: usize, document: String, first_page: u32, last_page: u32 },
    /// The model is answering: the characters written so far.
    Writing { characters: usize },
    /// What a part gave: new proposals, and what was dropped and why.
    Proposed { added: usize, dropped: Vec<String> },
    /// A step gave nothing; why.
    Failed { message: String },
    /// The work is over.
    End { stopped: bool },
}
