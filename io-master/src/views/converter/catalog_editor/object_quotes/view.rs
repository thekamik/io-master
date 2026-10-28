//! The data of the quotes of an object, as the page gets it.

use serde::Serialize;

use crate::model_old::evaluation_support::Quote;
use crate::model_old::standards::Document;

/// A quote of an object.
#[derive(Debug, Serialize)]
pub struct QuoteView {
    pub id: i64,
    pub source_document_id: i64,
    /// The title and part of the document quoted.
    pub document: String,
    pub page: u32,
    pub text: String,
    /// Whether the text was found on its page.
    pub found: bool,
}

/// The quotes of an object.
#[derive(Debug, Serialize)]
pub struct ObjectQuotesView {
    pub quotes: Vec<QuoteView>,
    /// The document the viewer opens for a new quote: that of the first
    /// quote, else the first document; `None` when no document is loaded.
    pub first_source: Option<i64>,
}

impl ObjectQuotesView {
    pub fn new(quotes: Vec<Quote>, documents: &[Document]) -> ObjectQuotesView {
        let mut views = Vec::with_capacity(quotes.len());

        for quote in quotes {
            views.push(QuoteView {
                id: quote.id,
                source_document_id: quote.source_document_id,
                document: title_of(documents, quote.source_document_id),
                page: quote.page,
                text: quote.text,
                found: quote.found,
            });
        }

        let first_source = match views.first() {
            Some(quote) => Some(quote.source_document_id),
            None => first_document(documents),
        };

        ObjectQuotesView { quotes: views, first_source }
    }
}

/// The title and part of document `id`, as the page shows it.
fn title_of(documents: &[Document], id: i64) -> String {
    for document in documents {
        if document.id == id {
            return document.title_and_part();
        }
    }

    "?".to_owned()
}

/// The first document, if one is loaded.
fn first_document(documents: &[Document]) -> Option<i64> {
    Some(documents.first()?.id)
}
