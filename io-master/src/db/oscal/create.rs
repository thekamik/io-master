//! Storing a new document of OSCAL, checked against the schema of NIST.
//! Its UUID must be new (`business/oscal/` checks it before, for a
//! message of its own; the key of the table refuses it as well).

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::oscal::OscalDocument;

use super::rows::checked_json;

/// Stores `document`.
pub fn document(db: &UserDb, document: &OscalDocument) -> Result<(), DbError> {
    let json = checked_json(document)?;
    let metadata = document.metadata();

    db.connection().execute(
        "INSERT INTO oscal_documents (uuid, model, title, version, oscal_version, last_modified, json) \
         VALUES (:uuid, :model, :title, :version, :oscal_version, :last_modified, :json)",
        named_params! {
            ":uuid": document.uuid(),
            ":model": document.model().name(),
            ":title": metadata.title,
            ":version": metadata.version,
            ":oscal_version": metadata.oscal_version,
            ":last_modified": metadata.last_modified,
            ":json": json,
        },
    )?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, update, delete).
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
