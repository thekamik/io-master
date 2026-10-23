//! Unit tests of `made.rs`.

use super::*;

use crate::test_support::standards::{database, load};

#[test]
fn a_catalog_is_made_from_the_documents_of_a_standard() {
    let test = database();
    let source = load(&test.db, b"%PDF catalog");

    let made = create(&test.db, "Common Criteria Part 3", "CC:2022 R1", &[source]).unwrap();
    let catalog = db_oscal::read::catalog(&test.db, &made.uuid).unwrap();

    assert_eq!(catalog.metadata.title, "Common Criteria Part 3");
    assert_eq!(catalog.metadata.oscal_version, OSCAL_VERSION);
    let resource = &catalog.back_matter.as_ref().unwrap().resources[0];
    assert_eq!(resource.title.as_deref(), Some("Common Criteria, Part 3 (CC:2022 R1)"));
    assert_eq!(resource.rlinks[0].hashes[0].algorithm, "SHA-256");

    assert!(matches!(create(&test.db, " ", "", &[]), Err(OscalError::Invalid(_))));
    assert!(matches!(create(&test.db, "X", "", &[source + 1]), Err(OscalError::Invalid(_))));
}
