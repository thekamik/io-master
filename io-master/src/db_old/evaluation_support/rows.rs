//! The rows of the evaluation support and the types they become
//! (`model/evaluation_support.rs`). Columns are read by their names; a
//! value the program never writes is `DbError::Corrupt`.

use rusqlite::Row;
use serde_json::{Value, json};

use crate::db_old::error::{corrupt, named};
use crate::model_old::evaluation_support::{Proposal, ProposalKind, Quote, QuoteDraft};

/// A quote from a row of `support_quotes`.
pub(super) fn quote_from_row(row: &Row<'_>) -> rusqlite::Result<Quote> {
    Ok(Quote {
        id: row.get("id")?,
        document_uuid: row.get("document_uuid")?,
        object_id: row.get("object_id")?,
        source_document_id: row.get("source_document_id")?,
        page: row.get("page")?,
        text: row.get("text")?,
        found: row.get("found")?,
    })
}

/// The quotes of a proposal as they are stored (JSON).
pub(super) fn quotes_json(quotes: &[QuoteDraft]) -> String {
    Value::Array(
        quotes
            .iter()
            .map(|quote| {
                json!({
                    "source_document_id": quote.source_document_id,
                    "page": quote.page,
                    "text": quote.text,
                    "found": quote.found,
                })
            })
            .collect(),
    )
    .to_string()
}

/// The quotes of a proposal from their JSON.
fn quotes_from_json(text: &str) -> Option<Vec<QuoteDraft>> {
    let value: Value = serde_json::from_str(text).ok()?;

    value
        .as_array()?
        .iter()
        .map(|quote| {
            Some(QuoteDraft {
                source_document_id: quote.get("source_document_id")?.as_i64()?,
                page: u32::try_from(quote.get("page")?.as_u64()?).ok()?,
                text: quote.get("text")?.as_str()?.to_owned(),
                found: quote.get("found")?.as_bool()?,
            })
        })
        .collect()
}

/// A proposal from a row of `support_proposals`.
pub(super) fn proposal_from_row(row: &Row<'_>) -> rusqlite::Result<Proposal> {
    let kind: String = row.get("kind")?;
    let content: String = row.get("content")?;
    let quotes: String = row.get("quotes")?;

    Ok(Proposal {
        id: row.get("id")?,
        document_uuid: row.get("document_uuid")?,
        kind: named("support_proposals", "kind", &kind, ProposalKind::from_name)?,
        parent_id: row.get("parent_id")?,
        content: serde_json::from_str(&content).map_err(|_| corrupt("support_proposals", "content", "not JSON"))?,
        quotes: quotes_from_json(&quotes).ok_or_else(|| corrupt("support_proposals", "quotes", "not the quotes"))?,
        note: row.get("note")?,
        model: row.get("model")?,
        created: row.get("created")?,
    })
}
