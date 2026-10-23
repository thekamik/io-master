//! Unit tests of `db/oscal/` (attached to `create.rs`; they cover the
//! whole folder).

use super::*;

use crate::db::oscal::{delete, read, update};
use crate::model::oscal::OscalModel;
use crate::test_support::standards::database;

fn catalog() -> OscalDocument {
    OscalDocument::from_json(include_str!("../../../oscal/examples/basic-catalog.json")).unwrap()
}

#[test]
fn a_document_is_stored_read_changed_and_deleted() {
    let test = database();
    let mut stored = catalog();
    let uuid = stored.uuid().to_owned();

    document(&test.db, &stored).unwrap();

    let summaries = read::summaries(&test.db).unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].model, OscalModel::Catalog);
    assert_eq!(read::document(&test.db, &uuid).unwrap().unwrap(), stored);

    // The same UUID twice is refused by the key of the table.
    assert!(document(&test.db, &stored).is_err());

    if let OscalDocument::Catalog(catalog) = &mut stored {
        catalog.metadata.title = "Changed".to_owned();
    }
    update::document(&test.db, &stored).unwrap();
    assert_eq!(read::summary(&test.db, &uuid).unwrap().unwrap().title, "Changed");

    assert!(delete::document(&test.db, &uuid).unwrap());
    assert!(read::document(&test.db, &uuid).unwrap().is_none());
    assert!(!delete::document(&test.db, &uuid).unwrap());
}

#[test]
fn a_document_that_breaks_the_schema_is_never_stored() {
    let test = database();
    let mut broken = catalog();

    // An empty list where OSCAL wants at least one item would be left
    // out; a title of the metadata that is not a string is not possible
    // in the types; so the UUID is broken.
    if let OscalDocument::Catalog(catalog) = &mut broken {
        catalog.uuid = "not a uuid".to_owned();
    }

    assert!(matches!(document(&test.db, &broken), Err(DbError::NotOscal(_))));
    assert!(read::summaries(&test.db).unwrap().is_empty());
}
