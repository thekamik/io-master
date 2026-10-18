//! Reading the evaluation support.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::evaluation_support::{Proposal, Quote};

use super::rows::{proposal_from_row, quote_from_row};

/// The quotes of document `document_uuid` (all of them, or of one
/// object), in the order they were stored.
pub fn quotes(db: &UserDb, document_uuid: &str, object_id: Option<&str>) -> Result<Vec<Quote>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT * FROM support_quotes WHERE document_uuid = :document AND (:object IS NULL OR object_id = :object) \
         ORDER BY id",
    )?;

    Ok(statement
        .query_map(named_params! { ":document": document_uuid, ":object": object_id }, quote_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// Whether a quote names document `source_document_id` of a standard.
pub fn quotes_source(db: &UserDb, source_document_id: i64) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT EXISTS (SELECT 1 FROM support_quotes WHERE source_document_id = :source)",
        named_params! { ":source": source_document_id },
        |row| row.get(0),
    )?)
}

/// The proposals waiting for document `document_uuid`, the oldest first.
pub fn proposals(db: &UserDb, document_uuid: &str) -> Result<Vec<Proposal>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT * FROM support_proposals WHERE document_uuid = :document ORDER BY id")?;

    Ok(statement
        .query_map(named_params! { ":document": document_uuid }, proposal_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// Proposal `id`; `None` when there is no such.
pub fn proposal(db: &UserDb, id: i64) -> Result<Option<Proposal>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT * FROM support_proposals WHERE id = :id", named_params! { ":id": id }, proposal_from_row)
        .optional()?)
}
