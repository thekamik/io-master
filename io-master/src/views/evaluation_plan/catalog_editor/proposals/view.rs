//! The data of the proposals, as the page gets it.

use serde::Serialize;
use serde_json::Value;

use crate::app::error::AppError;
use crate::components::part_rows::view::{self as part_rows, PartRowView};
use crate::model::evaluation_support::{Proposal, ProposalKind, QuoteDraft};
use crate::model::oscal::catalog::{Control, Group};
use crate::model::standards::Document;

/// A quote of a proposal, looked for on its page.
#[derive(Debug, Serialize)]
pub struct DraftView {
    pub source_document_id: i64,
    /// The title and part of the document quoted.
    pub document: String,
    pub page: u32,
    pub text: String,
    pub found: bool,
}

/// A proposal of the AI.
#[derive(Debug, Serialize)]
pub struct ProposalView {
    pub id: i64,
    /// `"group"` or `"control"`.
    pub kind: &'static str,
    /// Where it goes; `None`: the top of the catalog.
    pub parent_id: Option<String>,
    /// The id and title of the object proposed.
    pub object_id: String,
    pub title: String,
    /// Its parts, as rows.
    pub parts: Vec<PartRowView>,
    /// The object as its JSON of OSCAL, for a correction.
    pub content: Value,
    pub quotes: Vec<DraftView>,
    /// The explanation of the AI, never presented as a quote.
    pub note: String,
    pub model: String,
}

/// The views of `proposals`, in their order; the quotes with the titles of
/// their `documents`.
pub fn all(proposals: Vec<Proposal>, documents: &[Document]) -> Result<Vec<ProposalView>, AppError> {
    let mut views = Vec::with_capacity(proposals.len());

    for proposal in proposals {
        views.push(view(proposal, documents)?);
    }

    Ok(views)
}

fn view(proposal: Proposal, documents: &[Document]) -> Result<ProposalView, AppError> {
    let (object_id, title, parts) = object(proposal.kind, &proposal.content)?;
    let mut quotes = Vec::with_capacity(proposal.quotes.len());

    for quote in proposal.quotes {
        quotes.push(draft(quote, documents));
    }

    Ok(ProposalView {
        id: proposal.id,
        kind: proposal.kind.name(),
        parent_id: proposal.parent_id,
        object_id,
        title,
        parts,
        content: proposal.content,
        quotes,
        note: proposal.note,
        model: proposal.model,
    })
}

/// The id, the title and the parts of the object of a proposal.
fn object(kind: ProposalKind, content: &Value) -> Result<(String, String, Vec<PartRowView>), AppError> {
    let unreadable = "a proposal holds an object that is not OSCAL";

    match kind {
        ProposalKind::Group => {
            let Ok(group) = serde_json::from_value::<Group>(content.clone()) else {
                return Err(AppError::Failed(unreadable.to_owned()));
            };

            Ok((group.id.clone().unwrap_or_default(), group.title.clone(), part_rows::rows(&group.parts)?))
        }
        ProposalKind::Control => {
            let Ok(control) = serde_json::from_value::<Control>(content.clone()) else {
                return Err(AppError::Failed(unreadable.to_owned()));
            };

            Ok((control.id.clone(), control.title.clone(), part_rows::rows(&control.parts)?))
        }
    }
}

fn draft(quote: QuoteDraft, documents: &[Document]) -> DraftView {
    let mut document = "?".to_owned();

    for candidate in documents {
        if candidate.id == quote.source_document_id {
            document = candidate.title_and_part();
        }
    }

    DraftView {
        source_document_id: quote.source_document_id,
        document,
        page: quote.page,
        text: quote.text,
        found: quote.found,
    }
}
