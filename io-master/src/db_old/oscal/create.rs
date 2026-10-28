//! Storing a new document of OSCAL, checked against the schema of NIST.
//! Its UUID must be new (`business/oscal/` checks it before, for a
//! message of its own; the key of the table refuses it as well).

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::oscal::OscalDocument;

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

#[cfg(test)]
mod tests {
    //! Tests of storing a new document of OSCAL.

    use super::*;

    use crate::db_old::oscal::read;
    use crate::test_support::db::basic_catalog;
    use crate::test_support::standards::database;

    #[test]
    fn a_document_is_stored_as_it_was_given() {
        let test = database();
        let stored = basic_catalog();

        document(&test.db, &stored).unwrap();

        assert_eq!(read::document(&test.db, stored.uuid()).unwrap().unwrap(), stored);
    }

    #[test]
    fn the_same_uuid_twice_is_refused() {
        let test = database();
        document(&test.db, &basic_catalog()).unwrap();

        // The key of the table refuses it.
        assert!(document(&test.db, &basic_catalog()).is_err());
    }

    #[test]
    fn a_document_that_breaks_the_schema_is_never_stored() {
        let test = database();
        let mut broken = basic_catalog();

        // An empty list where OSCAL wants at least one item would be left
        // out; a title of the metadata that is not a string is not possible
        // in the types; so the UUID is broken.
        if let OscalDocument::Catalog(catalog) = &mut broken {
            catalog.uuid = "not a uuid".to_owned();
        }

        assert!(matches!(document(&test.db, &broken), Err(DbError::NotOscal(_))));
        assert!(read::summaries(&test.db).unwrap().is_empty());
    }
}
