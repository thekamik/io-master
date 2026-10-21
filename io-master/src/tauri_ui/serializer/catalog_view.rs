//! A catalog of OSCAL as the editor of the page shows it: the catalog
//! itself as its JSON of OSCAL (the page reads the format of NIST), and
//! beside it the evaluation support: the quotes of its objects and the
//! proposals of the AI waiting for the evaluator. Also the place of a new
//! object, and the events of a long work with the AI:
//!
//! ```text
//! {"event": "part", "index": 1, "count": 3, "document": "...", "first_page": 1, "last_page": 9}
//! {"event": "writing", "characters": 1830}                the model is answering
//! {"event": "proposed", "added": 7, "dropped": ["..."]}    what a part gave
//! {"event": "failed", "message": "..."}                   this part gave nothing
//! {"event": "end", "stopped": false}                      last
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::business::oscal::building::Progress;
use crate::business::oscal::catalogs::Place;
use crate::model::evaluation_support::{Proposal, Quote, QuoteDraft};
use crate::model::oscal::catalog::Catalog;

use super::super::error::UiError;
use crate::views::plans::oscal_documents::view::OscalDocumentView;

/// A quote of an object.
#[derive(Debug, Serialize)]
pub struct QuoteView {
    pub id: i64,
    pub object_id: String,
    pub source_document_id: i64,
    pub page: u32,
    pub text: String,
    pub found: bool,
}

impl From<Quote> for QuoteView {
    fn from(quote: Quote) -> QuoteView {
        QuoteView {
            id: quote.id,
            object_id: quote.object_id,
            source_document_id: quote.source_document_id,
            page: quote.page,
            text: quote.text,
            found: quote.found,
        }
    }
}

/// A quote of a proposal.
#[derive(Debug, Serialize)]
pub struct DraftView {
    pub source_document_id: i64,
    pub page: u32,
    pub text: String,
    pub found: bool,
}

impl From<QuoteDraft> for DraftView {
    fn from(quote: QuoteDraft) -> DraftView {
        DraftView {
            source_document_id: quote.source_document_id,
            page: quote.page,
            text: quote.text,
            found: quote.found,
        }
    }
}

/// A proposal of the AI.
#[derive(Debug, Serialize)]
pub struct ProposalView {
    pub id: i64,
    /// "group" or "control".
    pub kind: &'static str,
    pub parent_id: Option<String>,
    /// The group or the control, as its JSON of OSCAL.
    pub content: Value,
    pub quotes: Vec<DraftView>,
    pub note: String,
    pub model: String,
}

impl From<Proposal> for ProposalView {
    fn from(proposal: Proposal) -> ProposalView {
        ProposalView {
            id: proposal.id,
            kind: proposal.kind.name(),
            parent_id: proposal.parent_id,
            content: proposal.content,
            quotes: proposal.quotes.into_iter().map(DraftView::from).collect(),
            note: proposal.note,
            model: proposal.model,
        }
    }
}

/// A catalog open in the editor.
#[derive(Debug, Serialize)]
pub struct CatalogView {
    pub document: OscalDocumentView,
    /// The catalog, as the value of the key "catalog" of its file.
    pub catalog: Value,
    pub quotes: Vec<QuoteView>,
    pub proposals: Vec<ProposalView>,
}

impl CatalogView {
    pub fn new(
        document: OscalDocumentView,
        catalog: &Catalog,
        quotes: Vec<Quote>,
        proposals: Vec<Proposal>,
    ) -> Result<CatalogView, UiError> {
        Ok(CatalogView {
            document,
            catalog: serde_json::to_value(catalog).map_err(|error| UiError::Failed(error.to_string()))?,
            quotes: quotes.into_iter().map(QuoteView::from).collect(),
            proposals: proposals.into_iter().map(ProposalView::from).collect(),
        })
    }
}

/// Where a new object goes, as the page sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceForm {
    /// "top", "group" or "control".
    pub kind: String,
    #[serde(default)]
    pub id: Option<String>,
}

impl PlaceForm {
    pub fn into_place(self) -> Result<Place, UiError> {
        match (self.kind.as_str(), self.id) {
            ("top", _) => Ok(Place::Top),
            ("group", Some(id)) => Ok(Place::Group(id)),
            ("control", Some(id)) => Ok(Place::Control(id)),
            _ => Err(UiError::Invalid("the place of the object is not known".to_owned())),
        }
    }
}

/// An object of OSCAL sent by the page (a group or a control), read into
/// the types of the program.
pub fn object<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, UiError> {
    serde_json::from_value(value).map_err(|error| UiError::Invalid(format!("the object is not OSCAL: {error}")))
}

/// Pages of a document to read, as the page sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRange {
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
}

/// An event of a long work with the AI.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum BuildEvent {
    Part { index: usize, count: usize, document: String, first_page: u32, last_page: u32 },
    Writing { characters: usize },
    Proposed { added: usize, dropped: Vec<String> },
    Failed { message: String },
    End { stopped: bool },
}

impl From<Progress> for BuildEvent {
    fn from(progress: Progress) -> BuildEvent {
        match progress {
            Progress::Part { index, count, document, first_page, last_page } => {
                BuildEvent::Part { index, count, document, first_page, last_page }
            }
            Progress::Writing { characters } => BuildEvent::Writing { characters },
            Progress::Proposed { added, dropped } => BuildEvent::Proposed { added, dropped },
            Progress::Failed(message) => BuildEvent::Failed { message },
            Progress::End { stopped } => BuildEvent::End { stopped },
        }
    }
}
