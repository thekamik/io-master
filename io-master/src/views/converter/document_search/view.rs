//! The data of the search, as the page gets it.

use serde::Serialize;

use crate::model_old::standards::{Document, Hit};

/// A hit of a search.
#[derive(Debug, Serialize)]
pub struct HitView {
    pub document_id: i64,
    /// The title of the document and its part: "Common Criteria — Part 3".
    pub document: String,
    pub page: u32,
    /// The text around the hit, on one line.
    pub snippet: String,
}

/// The views of `hits`, in their order, each with the title of its
/// document (one of `documents`).
pub fn all(hits: Vec<Hit>, documents: &[Document]) -> Vec<HitView> {
    let mut views = Vec::with_capacity(hits.len());

    for hit in hits {
        views.push(HitView {
            document_id: hit.document_id,
            document: title_of(documents, hit.document_id),
            page: hit.page,
            snippet: hit.snippet,
        });
    }

    views
}

/// The title of document `id` and its part, as the page shows it.
fn title_of(documents: &[Document], id: i64) -> String {
    for document in documents {
        if document.id == id {
            return document.title_and_part();
        }
    }

    String::new()
}
