//! Replacing a document of OSCAL by its new version, checked against
//! the schema of NIST.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::oscal::OscalDocument;
use crate::model::oscal::catalog::Catalog;

use super::error::OscalError;
use super::ids::now_date_time;

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

/// Stores a changed catalog with a new `last-modified`, checked against
/// the schema of NIST.
pub fn catalog(db: &UserDb, mut catalog: Catalog) -> Result<(), OscalError> {
    catalog.metadata.last_modified = now_date_time();
    document(db, &OscalDocument::Catalog(catalog))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of replacing a document of OSCAL by its new version.

    use super::*;

    use crate::db::oscal::{create, read};
    use crate::test_support::db::basic_catalog;
    use crate::test_support::standards::database;

    #[test]
    fn a_changed_document_replaces_the_stored_one() {
        let test = database();
        let mut stored = basic_catalog();
        create::document(&test.db, &stored).unwrap();

        if let OscalDocument::Catalog(catalog) = &mut stored {
            catalog.metadata.title = "Changed".to_owned();
        }
        document(&test.db, &stored).unwrap();

        assert_eq!(read::summary(&test.db, stored.uuid()).unwrap().unwrap().title, "Changed");
    }
}
