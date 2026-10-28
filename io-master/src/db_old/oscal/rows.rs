//! The rows of the documents of OSCAL and the types they become
//! (`model/oscal.rs`). Columns are read by their names; a model the
//! program never writes, or a JSON that is no longer a document of
//! OSCAL (the file changed outside the program), is `DbError::Corrupt`.

use rusqlite::Row;
use serde_json::Value;

use crate::db_old::error::{DbError, corrupt, named};
use crate::model_old::oscal::{OscalDocument, OscalModel, OscalSummary};

const TABLE: &str = "oscal_documents";

/// What a list shows, from a row.
pub(super) fn summary_from_row(row: &Row<'_>) -> rusqlite::Result<OscalSummary> {
    let model: String = row.get("model")?;

    Ok(OscalSummary {
        uuid: row.get("uuid")?,
        model: named(TABLE, "model", &model, OscalModel::from_name)?,
        title: row.get("title")?,
        version: row.get("version")?,
        oscal_version: row.get("oscal_version")?,
        last_modified: row.get("last_modified")?,
    })
}

/// The document of a row, read and checked again.
pub(super) fn document_from_row(row: &Row<'_>) -> rusqlite::Result<OscalDocument> {
    let json: String = row.get("json")?;

    OscalDocument::from_json(&json).map_err(|_| corrupt(TABLE, "json", "a document that is not OSCAL"))
}

/// The JSON of a document, checked against the schema of NIST: what is
/// written into the table. Nothing that breaks the schema is ever
/// stored.
pub(super) fn checked_json(document: &OscalDocument) -> Result<String, DbError> {
    let value: Value = document.to_json().map_err(DbError::NotOscal)?;

    Ok(value.to_string())
}
