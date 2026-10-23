//! Unit tests of `file.rs`, on the examples of NIST.

use super::*;

use crate::model::oscal::OscalModel;
use crate::test_support::standards::database;

const CATALOG: &str = include_str!("../../../../oscal/examples/basic-catalog.json");
const PLAN: &str = include_str!("../../../../oscal/examples/ifa_assessment-plan-example.json");

#[test]
fn files_of_nist_are_imported_listed_exported_and_deleted() {
    let test = database();

    let catalog = import(&test.db, CATALOG).unwrap();
    let plan = import(&test.db, PLAN).unwrap();
    assert_eq!(catalog.model, OscalModel::Catalog);
    assert_eq!(plan.model, OscalModel::AssessmentPlan);
    assert_eq!(db_oscal::read::summaries(&test.db).unwrap().len(), 2);

    // The exported file is the document that was imported.
    let exported: serde_json::Value = serde_json::from_str(&export(&test.db, &catalog.uuid).unwrap()).unwrap();
    let mut original: serde_json::Value = serde_json::from_str(CATALOG).unwrap();
    original.as_object_mut().unwrap().remove("$schema");
    assert_eq!(exported, original);

    // The same document twice is refused, with the name of the first.
    assert!(matches!(import(&test.db, CATALOG), Err(OscalError::AlreadyStored(_))));

    assert!(delete(&test.db, &catalog.uuid).unwrap());
    assert!(matches!(export(&test.db, &catalog.uuid), Err(OscalError::NotFound)));
}

#[test]
fn a_file_that_is_not_oscal_is_refused() {
    let test = database();

    assert!(matches!(import(&test.db, "{\"catalog\": {}}"), Err(OscalError::NotOscal(NotOscal::Schema(_)))));
    assert!(matches!(import(&test.db, "%PDF-1.4"), Err(OscalError::NotOscal(NotOscal::NotJson(_)))));
    assert!(db_oscal::read::summaries(&test.db).unwrap().is_empty());
}
