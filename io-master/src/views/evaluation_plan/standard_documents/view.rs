//! The data of the list of documents of standards, as the page gets and
//! sends it, and the events of loading them.
//!
//! Loading reaches the page through a channel of Tauri, as these
//! events:
//!
//! ```text
//! {"event": "file", "name": "cc-part3.pdf", "index": 1, "count": 2}   a file starts
//! {"event": "pages", "done": 40, "total": 236}                          progress
//! {"event": "loaded", "document": {...}}                                 stored
//! {"event": "failed", "name": "x.pdf", "message": "..."}                 not loaded
//! {"event": "end", "stopped": false}                                     last
//! ```

use serde::{Deserialize, Serialize};

use crate::model::standards::{Document, DocumentFields};

/// A loaded document.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentView {
    pub id: i64,
    pub title: String,
    pub part: String,
    pub version: String,
    pub file_name: String,
    pub sha256: String,
    pub page_count: u32,
}

impl DocumentView {
    pub fn new(document: Document) -> DocumentView {
        DocumentView {
            id: document.id,
            title: document.fields.title,
            part: document.fields.part,
            version: document.fields.version,
            file_name: document.file_name,
            sha256: document.sha256,
            page_count: document.page_count,
        }
    }
}

/// What the page sends to change a document.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentForm {
    pub title: String,
    pub part: String,
    pub version: String,
}

impl DocumentForm {
    pub fn into_fields(self) -> DocumentFields {
        DocumentFields { title: self.title, part: self.part, version: self.version }
    }
}

/// An event of loading documents.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LoadEvent {
    /// A file starts (counted from 1).
    File { name: String, index: usize, count: usize },
    /// Pages read of the file.
    Pages { done: u32, total: u32 },
    /// The file is stored.
    Loaded { document: DocumentView },
    /// The file was not loaded.
    Failed { name: String, message: String },
    /// The loading is over.
    End { stopped: bool },
}

/// The views of `documents`, in their order.
pub fn all(documents: Vec<Document>) -> Vec<DocumentView> {
    let mut views = Vec::with_capacity(documents.len());

    for document in documents {
        views.push(DocumentView::new(document));
    }

    views
}
