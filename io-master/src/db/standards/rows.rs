//! The rows of the documents of standards and the Rust type they
//! become (`standards/`).

use rusqlite::Row;

use crate::model::standards::{Document, DocumentFields};

/// The columns of a document, in the order of [`document_from_row`].
/// The PDF and the pages are read on their own (`read.rs`).
pub(super) const COLUMNS: &str = "id, title, part, version, file_name, sha256, page_count";

pub(super) fn document_from_row(row: &Row<'_>) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        fields: DocumentFields { title: row.get(1)?, part: row.get(2)?, version: row.get(3)? },
        file_name: row.get(4)?,
        sha256: row.get(5)?,
        page_count: row.get(6)?,
    })
}
