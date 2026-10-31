//! The standards of the plans: an OSCAL catalog imported by the user,
//! checked against the schema of NIST before it is stored.
//!
//! ```text
//! the page ──the text of a file──► import_standard ──check_catalog (oscal.rs)──► create_oscal_standard_document
//! ```

use crate::db::models::evaluation::oscal_standard_document::create_oscal_standard_document;
use crate::db::storage::user_database::UserDatabase;
use crate::model::evaluation::oscal_standard_document::OscalStandardDocument;
use crate::panels::logic::evaluation_plan::error::PlanError;
use crate::panels::logic::evaluation_plan::oscal::check_catalog;

/// Imports the OSCAL catalog `text`: checked against the schema of NIST,
/// stored as it was given, with the title and the version of its metadata.
/// Gives its id.
///
/// Called by: `panels/views/evaluation_plan/` (the import of a standard).
/// Calls: `oscal::check_catalog`, `create_oscal_standard_document`.
pub fn import_standard(db: &UserDatabase, text: &str) -> Result<i64, PlanError> {
    let (_document, checked) = check_catalog(text)?;

    ensure!(!checked.title.is_empty(), PlanError::Refused("the catalog has no title".to_owned()));

    let standard =
        OscalStandardDocument { id: 0, title: checked.title, doc: text.to_owned(), version: checked.version };

    Ok(create_oscal_standard_document(db, &standard)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the import of a standard.

    use super::*;

    use crate::db::models::evaluation::oscal_standard_document::read_oscal_standard_document;
    use crate::test_support::database::database;

    const NIST_SAMPLE: &str = include_str!("../../../../oscal/examples/basic-catalog.json");

    #[test]
    fn a_catalog_is_imported_as_it_was_given() {
        let (_directory, db) = database();
        let id = import_standard(&db, NIST_SAMPLE).unwrap();
        let stored = read_oscal_standard_document(&db, id).unwrap();

        assert_eq!(stored.doc, NIST_SAMPLE);
        assert!(!stored.title.is_empty());
    }

    #[test]
    fn a_document_that_is_no_catalog_is_not_stored() {
        let (_directory, db) = database();

        assert!(matches!(import_standard(&db, "{\"catalog\": {}}"), Err(PlanError::Oscal(_))));
        assert!(matches!(import_standard(&db, "[]"), Err(PlanError::Oscal(_))));
    }
}
