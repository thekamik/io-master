//! Unit tests of the CRUD of an open test project: `schemes/`,
//! `tiles/`, `links/` and `content.rs`, by their rules.
//!
//! The tests call the CRUD through `test_support::single`: each call
//! opens the project from the database, makes the change on it and
//! checks that memory and the database are still the same.

use crate::test_projects::model::{
    External, Link, OutputSection, PageSpan, ProposalState, Source, Tile, TileFields, TileKind,
};
use crate::test_projects::rules::{SchemeError, StoreError};
use crate::test_projects::test_support::single::create_project;
use crate::test_projects::test_support::single::*;
use crate::test_projects::test_support::{TestDb, database, load, project_fields, project_with_scheme, scheme_fields};

/// A database with the small standard and a project with one scheme;
/// returns the project, the scheme and the document.
fn ready() -> (TestDb, i64, i64, i64) {
    let test = database();
    let document = load(&test.db, b"%PDF scheme");
    let (project, scheme) = project_with_scheme(&test.db, document);

    (test, project, scheme, document)
}

fn source(document: i64, page: u32, quote: &str) -> Source {
    Source { document_id: document, page, quote: quote.to_owned(), found: false }
}

#[test]
fn schemes_are_made_in_order_changed_accepted_and_deleted() {
    let (test, project, first, document) = ready();

    let mut proposed = scheme_fields("6", "ATE");
    proposed.state = ProposalState::Proposed;
    proposed.pages = Some(PageSpan { document_id: document, first_page: 2, last_page: 3 });
    let second = create(&test.db, project, &proposed).unwrap();

    let listed = of_project(&test.db, project).unwrap();
    assert_eq!(listed.iter().map(|scheme| scheme.id).collect::<Vec<_>>(), [first, second]);
    assert_eq!(listed[1].fields, proposed);

    accept(&test.db, second).unwrap();
    assert_eq!(get(&test.db, second).unwrap().unwrap().fields.state, ProposalState::Accepted);

    let mut renamed = scheme_fields("6", "Tests");
    renamed.pages = proposed.pages;
    update(&test.db, second, &renamed).unwrap();
    assert_eq!(get(&test.db, second).unwrap().unwrap().fields.name, "Tests");

    assert!(delete(&test.db, second).unwrap());
    assert_eq!(of_project(&test.db, project).unwrap().len(), 1);
}

#[test]
fn a_scheme_keeps_its_number_unique_and_its_pages_in_the_project() {
    let (test, project, _, document) = ready();

    assert!(matches!(
        create(&test.db, project, &scheme_fields("5", "Again")),
        Err(StoreError::Scheme(SchemeError::Invalid("number", _)))
    ));
    // Schemes without a number are many.
    create(&test.db, project, &scheme_fields("", "Annex")).unwrap();
    create(&test.db, project, &scheme_fields("", "Annex B")).unwrap();

    let mut past = scheme_fields("7", "Past the end");
    past.pages = Some(PageSpan { document_id: document, first_page: 2, last_page: 9 });
    assert!(matches!(create(&test.db, project, &past), Err(StoreError::Scheme(SchemeError::Invalid("pages", _)))));

    let other = load(&test.db, b"%PDF other");
    past.pages = Some(PageSpan { document_id: other, first_page: 1, last_page: 1 });
    assert!(matches!(create(&test.db, project, &past), Err(StoreError::Scheme(SchemeError::Invalid("pages", _)))));
}

#[test]
fn tiles_keep_their_fields_and_their_quotes_are_found() {
    let (test, _, scheme, document) = ready();

    let mut requirement = TileFields::new(TileKind::Requirement, "Complete functional specification");
    requirement.identifier = "ADV_FSP.4".to_owned();
    requirement.place = "Part 3, ADV_FSP".to_owned();
    requirement.sources = vec![
        source(document, 2, "The developer shall provide a functional specification."),
        source(document, 2, "The developer shall provide a full design."),
    ];
    requirement.x = 120.5;

    let id = add_tile(&test.db, scheme, &requirement).unwrap();
    let stored = tile(&test.db, id).unwrap().unwrap();

    assert_eq!(stored.scheme, scheme);
    assert_eq!(stored.fields.identifier, "ADV_FSP.4");
    assert_eq!(stored.fields.x, 120.5);
    // The first quote is on the page (across the divided word), the
    // second is made up.
    assert!(stored.fields.sources[0].found);
    assert!(!stored.fields.sources[1].found);
}

#[test]
fn sources_must_be_in_documents_of_the_project_and_on_their_pages() {
    let (test, _, scheme, document) = ready();
    let other = load(&test.db, b"%PDF other");

    let mut tile = TileFields::new(TileKind::Requirement, "R");
    tile.sources = vec![source(other, 1, "Scope")];
    assert!(matches!(add_tile(&test.db, scheme, &tile), Err(StoreError::Scheme(SchemeError::Invalid("sources", _)))));

    tile.sources = vec![source(document, 9, "Scope")];
    assert!(matches!(add_tile(&test.db, scheme, &tile), Err(StoreError::Scheme(SchemeError::Invalid("sources", _)))));
}

#[test]
fn inputs_outputs_and_external_inputs_are_stored() {
    let (test, _, scheme, _) = ready();

    let mut external = TileFields::new(TileKind::Input, "Flaw remediation procedures");
    external.external = Some(External { document: "CC Part 3".to_owned(), place: "ALC_FLR.2".to_owned() });
    external.state = ProposalState::Proposed;

    let mut output = TileFields::new(TileKind::Output, "Report of ADV");
    output.sections = vec![OutputSection::Requirements, OutputSection::Verdict];
    output.instructions = "One page per requirement.".to_owned();

    let external_id = add_tile(&test.db, scheme, &external).unwrap();
    let output_id = add_tile(&test.db, scheme, &output).unwrap();

    assert_eq!(tile(&test.db, external_id).unwrap().unwrap().fields, external);
    assert_eq!(tile(&test.db, output_id).unwrap().unwrap().fields, output);

    accept_tile(&test.db, external_id).unwrap();
    assert_eq!(tile(&test.db, external_id).unwrap().unwrap().fields.state, ProposalState::Accepted);
}

#[test]
fn links_follow_the_rules_and_go_with_their_tiles() {
    let (test, project, scheme, _) = ready();

    let input = add_tile(&test.db, scheme, &TileFields::new(TileKind::Input, "Functional specification")).unwrap();
    let requirement = add_tile(&test.db, scheme, &TileFields::new(TileKind::Requirement, "ADV_FSP.4")).unwrap();
    let output = add_tile(&test.db, scheme, &TileFields::new(TileKind::Output, "Report")).unwrap();

    let needed = add_link(&test.db, input, requirement).unwrap();
    add_link(&test.db, requirement, output).unwrap();

    assert!(matches!(add_link(&test.db, input, output), Err(StoreError::Scheme(SchemeError::LinkNotAllowed(..)))));
    assert!(matches!(add_link(&test.db, requirement, input), Err(StoreError::Scheme(SchemeError::Cycle))));

    // A linked tile keeps its kind.
    let mut changed = TileFields::new(TileKind::Output, "Functional specification");
    changed.state = ProposalState::Accepted;
    assert!(matches!(update_tile(&test.db, input, &changed), Err(StoreError::Scheme(SchemeError::Invalid("kind", _)))));

    assert!(delete_link(&test.db, needed).unwrap());
    assert_eq!(links(&test.db, project).unwrap().len(), 1);

    // Deleting a tile takes its links with it.
    assert!(delete_tile(&test.db, requirement).unwrap());
    assert!(links(&test.db, project).unwrap().is_empty());
    assert_eq!(project_tiles(&test.db, project).unwrap().len(), 2);
}

#[test]
fn links_join_schemes_of_one_project_and_their_circles_are_found() {
    let (test, project, first, document) = ready();
    let second = create(&test.db, project, &scheme_fields("6", "ATE")).unwrap();

    let earlier = add_tile(&test.db, first, &TileFields::new(TileKind::Requirement, "ADV_FSP.4")).unwrap();
    let later = add_tile(&test.db, second, &TileFields::new(TileKind::Requirement, "ATE_IND.2")).unwrap();

    // A requirement of one chapter must be met before one of another.
    add_link(&test.db, earlier, later).unwrap();
    assert!(matches!(add_link(&test.db, later, earlier), Err(StoreError::Scheme(SchemeError::Cycle))));

    // Tiles of two projects are never linked.
    let other = create_project(&test.db, &project_fields("Other", vec![document])).unwrap();
    let elsewhere = create(&test.db, other, &scheme_fields("1", "X")).unwrap();
    let foreign = add_tile(&test.db, elsewhere, &TileFields::new(TileKind::Requirement, "Y")).unwrap();
    assert!(matches!(add_link(&test.db, earlier, foreign), Err(StoreError::Scheme(SchemeError::Invalid("link", _)))));

    // Deleting a scheme takes the links of its tiles, also those to
    // other schemes.
    assert!(delete(&test.db, second).unwrap());
    assert!(links(&test.db, project).unwrap().is_empty());
    assert!(tile(&test.db, later).unwrap().is_none());
}

#[test]
fn tiles_move_on_the_canvas_and_between_schemes() {
    let (test, project, scheme, document) = ready();
    let second = create(&test.db, project, &scheme_fields("6", "ATE")).unwrap();
    let id = add_tile(&test.db, scheme, &TileFields::new(TileKind::Input, "Sample")).unwrap();
    let requirement = add_tile(&test.db, scheme, &TileFields::new(TileKind::Requirement, "R")).unwrap();
    add_link(&test.db, id, requirement).unwrap();

    move_tile(&test.db, id, 40.0, -12.5).unwrap();

    let moved = tile(&test.db, id).unwrap().unwrap();
    assert_eq!((moved.fields.x, moved.fields.y), (40.0, -12.5));
    assert!(matches!(move_tile(&test.db, id, f64::NAN, 0.0), Err(StoreError::Scheme(_))));
    assert!(matches!(move_tile(&test.db, 99, 0.0, 0.0), Err(StoreError::Scheme(SchemeError::NotFound("tile")))));

    // To another scheme: the link stays.
    move_to_scheme(&test.db, &[id], second).unwrap();
    assert_eq!(tile(&test.db, id).unwrap().unwrap().scheme, second);
    assert_eq!(links(&test.db, project).unwrap().len(), 1);

    // Never to a scheme of another project.
    let other = create_project(&test.db, &project_fields("Other", vec![document])).unwrap();
    let elsewhere = create(&test.db, other, &scheme_fields("1", "X")).unwrap();
    assert!(move_to_scheme(&test.db, &[id], elsewhere).is_err());
    assert_eq!(tile(&test.db, id).unwrap().unwrap().scheme, second);
}

#[test]
fn the_content_is_replaced_by_an_earlier_one() {
    let (test, project, scheme, document) = ready();
    let second = create(&test.db, project, &scheme_fields("6", "ATE")).unwrap();

    let input = add_tile(&test.db, scheme, &TileFields::new(TileKind::Input, "Specification")).unwrap();
    let mut traced = TileFields::new(TileKind::Requirement, "ADV_FSP.4");
    traced.sources = vec![source(document, 2, "The developer shall provide")];
    let requirement = add_tile(&test.db, scheme, &traced).unwrap();
    let later = add_tile(&test.db, second, &TileFields::new(TileKind::Requirement, "ATE_IND.2")).unwrap();
    add_link(&test.db, input, requirement).unwrap();
    add_link(&test.db, requirement, later).unwrap();

    let before = crate::test_projects::projects::read::data(&test.db, project).unwrap().unwrap();

    // A change, then back.
    delete_tile(&test.db, input).unwrap();
    move_to_scheme(&test.db, &[requirement], second).unwrap();
    add_tile(&test.db, scheme, &TileFields::new(TileKind::Output, "Report")).unwrap();

    let numbers = replace_content(&test.db, project, &before.tiles, &before.links).unwrap();
    let after = crate::test_projects::projects::read::data(&test.db, project).unwrap().unwrap();

    assert_eq!(numbers.len(), 3);
    assert_eq!(after.tiles.len(), 3);
    assert_eq!(after.tiles[1].fields, before.tiles[1].fields);
    assert_eq!(after.tiles[1].scheme, scheme);
    assert_eq!(after.tiles[2].scheme, second);
    assert!(after.tiles[1].fields.sources[0].found);
    assert_eq!(after.links.len(), 2);
    assert_eq!(after.links[0].from, numbers[0].1);
}

#[test]
fn a_content_that_breaks_the_rules_is_not_written() {
    let (test, project, scheme, _) = ready();
    let input = Tile { id: 1, scheme, fields: TileFields::new(TileKind::Input, "A") };
    let output = Tile { id: 2, scheme, fields: TileFields::new(TileKind::Output, "B") };

    let result = replace_content(&test.db, project, &[input.clone(), output], &[Link { id: 1, from: 1, to: 2 }]);
    assert!(matches!(result, Err(StoreError::Scheme(SchemeError::LinkNotAllowed(..)))));
    assert!(project_tiles(&test.db, project).unwrap().is_empty());

    // A tile of a scheme the project does not have.
    let lost = Tile { scheme: 999, ..input };
    assert!(matches!(
        replace_content(&test.db, project, &[lost], &[]),
        Err(StoreError::Scheme(SchemeError::NotFound("scheme")))
    ));
}
