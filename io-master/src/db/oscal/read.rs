//! Reading the documents of OSCAL.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::oscal::catalog::Catalog;
use crate::model::oscal::{OscalDocument, OscalSummary};

use super::error::OscalError;

use super::rows::{document_from_row, summary_from_row};

/// Every document, by model and title.
pub fn summaries(db: &UserDb) -> Result<Vec<OscalSummary>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT uuid, model, title, version, oscal_version, last_modified FROM oscal_documents \
         ORDER BY model, title COLLATE NOCASE, uuid",
    )?;

    Ok(statement.query_map([], summary_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// What identifies document `uuid`; `None` when there is no such.
pub fn summary(db: &UserDb, uuid: &str) -> Result<Option<OscalSummary>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT uuid, model, title, version, oscal_version, last_modified FROM oscal_documents WHERE uuid = :uuid",
            named_params! { ":uuid": uuid },
            summary_from_row,
        )
        .optional()?)
}

/// Document `uuid`; `None` when there is no such.
pub fn document(db: &UserDb, uuid: &str) -> Result<Option<OscalDocument>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT json FROM oscal_documents WHERE uuid = :uuid",
            named_params! { ":uuid": uuid },
            document_from_row,
        )
        .optional()?)
}

/// Catalog `uuid`; an error when there is no such document, or it is not
/// a catalog.
pub fn catalog(db: &UserDb, uuid: &str) -> Result<Catalog, OscalError> {
    match document(db, uuid)? {
        Some(OscalDocument::Catalog(catalog)) => Ok(catalog),
        Some(_) => Err(OscalError::WrongModel),
        None => Err(OscalError::NotFound),
    }
}
