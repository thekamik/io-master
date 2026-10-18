//! The documents of standards as the page shows them, and the events
//! of loading them.
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

use crate::business::standards::load::LoadProgress;
use serde::{Deserialize, Serialize};

use crate::model::standards::{Document, DocumentFields, Hit};

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

impl From<Document> for DocumentView {
    fn from(document: Document) -> DocumentView {
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

impl From<DocumentForm> for DocumentFields {
    fn from(form: DocumentForm) -> DocumentFields {
        DocumentFields { title: form.title, part: form.part, version: form.version }
    }
}

/// A hit of a search.
#[derive(Debug, Serialize)]
pub struct HitView {
    pub document_id: i64,
    pub page: u32,
    pub snippet: String,
}

impl From<Hit> for HitView {
    fn from(hit: Hit) -> HitView {
        HitView { document_id: hit.document_id, page: hit.page, snippet: hit.snippet }
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

impl From<LoadProgress> for LoadEvent {
    fn from(progress: LoadProgress) -> LoadEvent {
        match progress {
            LoadProgress::File { name, index, count } => LoadEvent::File { name, index, count },
            LoadProgress::Pages { done, total } => LoadEvent::Pages { done, total },
            LoadProgress::Loaded(document) => LoadEvent::Loaded { document: document.into() },
            LoadProgress::Failed { name, message } => LoadEvent::Failed { name, message },
            LoadProgress::End { stopped } => LoadEvent::End { stopped },
        }
    }
}
