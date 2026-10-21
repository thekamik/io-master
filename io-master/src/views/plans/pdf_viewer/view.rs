//! The data of the viewer, as the page gets it: the documents it can
//! show (the PDF itself comes as bytes).

use serde::Serialize;

use crate::model::standards::Document;

/// A document the viewer can show.
#[derive(Debug, Serialize)]
pub struct ViewerDocumentView {
    pub id: i64,
    pub title: String,
    pub part: String,
    pub page_count: u32,
}

/// The views of `documents`, in their order.
pub fn all(documents: Vec<Document>) -> Vec<ViewerDocumentView> {
    let mut views = Vec::with_capacity(documents.len());

    for document in documents {
        views.push(ViewerDocumentView {
            id: document.id,
            title: document.fields.title,
            part: document.fields.part,
            page_count: document.page_count,
        });
    }

    views
}
