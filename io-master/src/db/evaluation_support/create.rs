//! Storing quotes and proposals of the evaluation support.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_support::{ProposalKind, QuoteDraft};

use super::rows::quotes_json;

/// Stores a quote of object `object_id` of document `document_uuid`
/// (already looked for on its page); returns its number.
pub fn quote(db: &UserDb, document_uuid: &str, object_id: &str, quote: &QuoteDraft) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO support_quotes (document_uuid, object_id, source_document_id, page, text, found) \
         VALUES (:document, :object, :source, :page, :text, :found)",
        named_params! {
            ":document": document_uuid,
            ":object": object_id,
            ":source": quote.source_document_id,
            ":page": quote.page,
            ":text": quote.text,
            ":found": quote.found,
        },
    )?;

    Ok(db.connection().last_insert_rowid())
}

/// A proposal of the AI, as it is stored.
pub struct NewProposal<'a> {
    pub document_uuid: &'a str,
    pub kind: ProposalKind,
    pub parent_id: Option<&'a str>,
    pub content: &'a serde_json::Value,
    pub quotes: &'a [QuoteDraft],
    pub note: &'a str,
    pub model: &'a str,
}

/// Stores a proposal; returns its number.
pub fn proposal(db: &UserDb, proposal: &NewProposal<'_>) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO support_proposals (document_uuid, kind, parent_id, content, quotes, note, model) \
         VALUES (:document, :kind, :parent, :content, :quotes, :note, :model)",
        named_params! {
            ":document": proposal.document_uuid,
            ":kind": proposal.kind.name(),
            ":parent": proposal.parent_id,
            ":content": proposal.content.to_string(),
            ":quotes": quotes_json(proposal.quotes),
            ":note": proposal.note,
            ":model": proposal.model,
        },
    )?;

    Ok(db.connection().last_insert_rowid())
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, delete).
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
