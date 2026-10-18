//! Unit tests of `catalogs.rs` (and of `quotes.rs`).

use super::*;

use crate::business::oscal::{documents, quotes};
use crate::business::standards::test_support::{database, load};
use crate::model::oscal::common::{Part, Property};

fn group(id: &str, title: &str) -> Group {
    Group {
        id: Some(id.to_owned()),
        class: None,
        title: title.to_owned(),
        params: Vec::new(),
        props: Vec::new(),
        links: Vec::new(),
        parts: Vec::new(),
        groups: Vec::new(),
        controls: Vec::new(),
        other: Default::default(),
    }
}

fn control(id: &str, title: &str, statement: &str) -> Control {
    Control {
        id: id.to_owned(),
        class: None,
        title: title.to_owned(),
        params: Vec::new(),
        props: vec![Property {
            name: "label".to_owned(),
            value: id.to_owned(),
            ns: None,
            class: None,
            other: Default::default(),
        }],
        links: Vec::new(),
        parts: vec![Part {
            id: Some(format!("{id}_smt")),
            name: "statement".to_owned(),
            ns: None,
            class: None,
            title: None,
            props: Vec::new(),
            prose: Some(statement.to_owned()),
            parts: Vec::new(),
            links: Vec::new(),
            other: Default::default(),
        }],
        controls: Vec::new(),
        other: Default::default(),
    }
}

#[test]
fn a_catalog_is_made_from_the_documents_of_a_standard() {
    let test = database();
    let source = load(&test.db, b"%PDF catalog");

    let made = create(&test.db, "Common Criteria Part 3", "CC:2022 R1", &[source]).unwrap();
    let catalog = get(&test.db, &made.uuid).unwrap();

    assert_eq!(catalog.metadata.title, "Common Criteria Part 3");
    assert_eq!(catalog.metadata.oscal_version, OSCAL_VERSION);
    let resource = &catalog.back_matter.as_ref().unwrap().resources[0];
    assert_eq!(resource.rlinks[0].hashes[0].algorithm, "SHA-256");
    assert_eq!(documents::list(&test.db).unwrap().len(), 1);

    assert!(matches!(create(&test.db, " ", "", &[]), Err(OscalError::Invalid(_))));
}

#[test]
fn groups_and_controls_are_added_changed_and_deleted() {
    let test = database();
    let uuid = create(&test.db, "CC Part 3", "", &[]).unwrap().uuid;

    add_group(&test.db, &uuid, &Place::Top, group("ADV", "Development")).unwrap();
    add_group(&test.db, &uuid, &Place::Group("ADV".to_owned()), group("ADV_FSP", "Functional specification")).unwrap();
    add_control(
        &test.db,
        &uuid,
        &Place::Group("ADV_FSP".to_owned()),
        control("ADV_FSP.4", "Complete functional specification", "The developer shall provide ..."),
    )
    .unwrap();

    // Every id once.
    assert!(matches!(
        add_control(&test.db, &uuid, &Place::Top, control("ADV_FSP.4", "Again", "x")),
        Err(OscalError::IdTaken(_))
    ));
    assert!(matches!(
        add_control(&test.db, &uuid, &Place::Group("ATE".to_owned()), control("ATE_IND.2", "T", "x")),
        Err(OscalError::NoSuchObject(_))
    ));

    // A change keeps the content of the object.
    update_group(&test.db, &uuid, "ADV_FSP", group("ADV_FSP", "FSP")).unwrap();
    update_control(&test.db, &uuid, "ADV_FSP.4", control("ADV_FSP.4", "Complete", "Changed.")).unwrap();

    let catalog = get(&test.db, &uuid).unwrap();
    let all = catalog.all_controls();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, ["Development", "FSP"]);
    assert_eq!(all[0].1.parts[0].prose.as_deref(), Some("Changed."));

    // Deleting a group takes its content and the quotes of it.
    delete_object(&test.db, &uuid, "ADV").unwrap();
    assert!(get(&test.db, &uuid).unwrap().all_controls().is_empty());
    assert!(matches!(delete_object(&test.db, &uuid, "ADV"), Err(OscalError::NoSuchObject(_))));
}

#[test]
fn quotes_are_looked_for_on_their_pages_and_go_with_their_object() {
    let test = database();
    let source = load(&test.db, b"%PDF quotes");
    let uuid = create(&test.db, "CC Part 3", "", &[source]).unwrap().uuid;
    add_control(&test.db, &uuid, &Place::Top, control("ADV_FSP.4", "Complete functional specification", "x")).unwrap();

    quotes::create(&test.db, &uuid, "ADV_FSP.4", source, 2, "The developer shall provide a functional specification.")
        .unwrap();
    quotes::create(&test.db, &uuid, "ADV_FSP.4", source, 2, "The evaluator shall dance.").unwrap();

    let stored = quotes::list(&test.db, &uuid, Some("ADV_FSP.4")).unwrap();
    assert!(stored[0].found, "across the divided word");
    assert!(!stored[1].found, "made up");

    // A document of a standard that a quote names stays.
    assert!(crate::business::standards::documents::delete(&test.db, source).is_err());

    delete_object(&test.db, &uuid, "ADV_FSP.4").unwrap();
    assert!(quotes::list(&test.db, &uuid, None).unwrap().is_empty());
}
