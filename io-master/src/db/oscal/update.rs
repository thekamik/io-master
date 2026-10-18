//! Replacing a document of OSCAL by its new version, checked against
//! the schema of NIST.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::oscal::OscalDocument;

use super::rows::checked_json;

/// Replaces the stored document with the UUID of `document`.
pub fn document(db: &UserDb, document: &OscalDocument) -> Result<(), DbError> {
    let json = checked_json(document)?;
    let metadata = document.metadata();

    let count = db.connection().execute(
        "UPDATE oscal_documents SET model = :model, title = :title, version = :version, \
         oscal_version = :oscal_version, last_modified = :last_modified, json = :json WHERE uuid = :uuid",
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

    if count == 0 { Err(DbError::NoRow { table: "oscal_documents", id: 0 }) } else { Ok(()) }
}
