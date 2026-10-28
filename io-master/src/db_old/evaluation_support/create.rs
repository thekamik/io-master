//! Storing quotes and proposals of the evaluation support.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::evaluation_support::{ProposalKind, QuoteDraft};

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

#[cfg(test)]
mod tests {
    //! Tests of storing quotes and proposals of the evaluation support.

    use super::*;

    use crate::db_old::evaluation_support::read;
    use crate::test_support::db::{proposal_content, quote_draft, stored_proposal};
    use crate::test_support::standards::{database, load};

    #[test]
    fn a_quote_is_stored_for_its_object() {
        let test = database();
        let source = load(&test.db, b"%PDF support");

        let id = quote(&test.db, "doc", "ADV_FSP.4", &quote_draft(source, "The developer shall")).unwrap();

        let stored = read::quotes(&test.db, "doc", Some("ADV_FSP.4")).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, id);
    }

    #[test]
    fn a_proposal_keeps_its_content_its_quotes_and_its_parent() {
        let test = database();
        let source = load(&test.db, b"%PDF proposals");

        let id = stored_proposal(&test.db, source);

        let stored = read::proposal(&test.db, id).unwrap().unwrap();
        assert_eq!(stored.content, proposal_content());
        assert_eq!(stored.quotes, [QuoteDraft { found: false, ..quote_draft(source, "ADV_FSP.4") }]);
        assert_eq!(stored.parent_id.as_deref(), Some("ADV"));
    }
}
