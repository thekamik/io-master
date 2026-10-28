//! The data of a new catalog, as the page gets it: the documents it can
//! come from.

use serde::Serialize;

use crate::model_old::standards::Document;

/// A document a catalog can come from.
#[derive(Debug, Serialize)]
pub struct SourceView {
    pub id: i64,
    pub title: String,
    pub part: String,
}

/// The views of `documents`, in their order.
pub fn all(documents: Vec<Document>) -> Vec<SourceView> {
    let mut views = Vec::with_capacity(documents.len());

    for document in documents {
        views.push(SourceView { id: document.id, title: document.fields.title, part: document.fields.part });
    }

    views
}
