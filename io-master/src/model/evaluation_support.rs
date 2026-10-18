//! The evaluation support (decided by the user, 2026-10-18): what the
//! program adds to the documents of OSCAL, kept apart from them and
//! linked to their objects only by the UUID of the document and the id
//! of the object (a group, a control, a part). Nothing of it is ever
//! written into an OSCAL document.
//!
//! ```text
//! Quote      a passage of a document of a standard (PDF) an object
//!            comes from: the page, the text, and whether the program
//!            found it on that page
//! Proposal   what the AI proposes for a document (a group or a control,
//!            as an OSCAL fragment, with its quotes and its note); it
//!            enters the document only when the evaluator accepts it
//! ```

use serde_json::Value;

/// A passage of a document of a standard that an object of an OSCAL
/// document comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub id: i64,
    /// The OSCAL document.
    pub document_uuid: String,
    /// The id of the object in it (a group, a control, a part).
    pub object_id: String,
    /// The document of a standard (PDF) quoted (`model/standards.rs`).
    pub source_document_id: i64,
    /// The page, counted from 1.
    pub page: u32,
    pub text: String,
    /// Whether the program found the text on that page.
    pub found: bool,
}

/// What a proposal of the AI adds to a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalKind {
    Group,
    Control,
}

impl ProposalKind {
    pub fn name(self) -> &'static str {
        match self {
            ProposalKind::Group => "group",
            ProposalKind::Control => "control",
        }
    }

    pub fn from_name(name: &str) -> Option<ProposalKind> {
        match name {
            "group" => Some(ProposalKind::Group),
            "control" => Some(ProposalKind::Control),
            _ => None,
        }
    }
}

/// A passage the AI names, before it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteDraft {
    pub source_document_id: i64,
    pub page: u32,
    pub text: String,
    pub found: bool,
}

/// A proposal of the AI for an OSCAL document, waiting for the
/// evaluator.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub id: i64,
    pub document_uuid: String,
    pub kind: ProposalKind,
    /// The id of the group (or control) it goes into; `None`: the top
    /// of the catalog.
    pub parent_id: Option<String>,
    /// The group or the control, as its JSON of OSCAL.
    pub content: Value,
    /// The passages it comes from, checked on their pages.
    pub quotes: Vec<QuoteDraft>,
    /// The note of the AI: its explanation, never presented as a quote.
    pub note: String,
    /// The model that proposed it.
    pub model: String,
    /// Seconds since 1970 (UTC).
    pub created: i64,
}
