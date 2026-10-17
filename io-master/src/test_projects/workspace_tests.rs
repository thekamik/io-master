//! Unit tests of `workspace.rs`: the open project in memory stays the
//! same as the database, and "Undo" brings versions back.

use super::*;

use crate::test_projects::model::{TileFields, TileKind};
use crate::test_projects::test_support::{database, load, opened, project_fields, project_with_scheme};
use crate::test_projects::{links, projects, schemes, tiles};

/// Whether the project in memory is the same as in the database.
fn same_as_stored(db: &UserDb, open: &OpenProject) -> bool {
    &projects::read::data(db, open.data().project.id).unwrap().unwrap() == open.data()
}

#[test]
fn every_change_is_written_through_and_kept_in_memory() {
    let test = database();
    let document = load(&test.db, b"%PDF workspace");
    let (project, scheme) = project_with_scheme(&test.db, document);
    let mut open = opened(&test.db, project);

    let input = open
        .change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Input, "  Sample  ")))
        .unwrap();
    let requirement = open
        .change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Requirement, "Test")))
        .unwrap();
    open.change(|open| links::create::link(&test.db, open, input, requirement)).unwrap();
    open.change(|open| tiles::update::position(&test.db, open, input, 10.0, 20.0)).unwrap();

    assert_eq!(open.data().tiles.len(), 2);
    assert_eq!(tiles::read::get(&open, input).unwrap().fields.title, "Sample");
    assert!(same_as_stored(&test.db, &open));
}

#[test]
fn a_refused_change_changes_nothing_and_keeps_no_version() {
    let test = database();
    let document = load(&test.db, b"%PDF refused");
    let (project, scheme) = project_with_scheme(&test.db, document);
    let mut open = opened(&test.db, project);

    let refused = open.change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Input, " ")));

    assert!(refused.is_err());
    assert!(open.data().tiles.is_empty());
    assert!(!open.can_undo());
}

#[test]
fn undo_brings_back_the_tiles_and_links_with_their_numbers() {
    let test = database();
    let document = load(&test.db, b"%PDF undo");
    let (project, scheme) = project_with_scheme(&test.db, document);
    let mut open = opened(&test.db, project);

    let input = open
        .change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Input, "Sample")))
        .unwrap();
    let requirement = open
        .change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Requirement, "Test")))
        .unwrap();
    open.change(|open| links::create::link(&test.db, open, input, requirement)).unwrap();
    let before = open.data().clone();

    open.change(|open| tiles::delete::tile(&test.db, open, input)).unwrap();
    assert_eq!(open.data().tiles.len(), 1);
    assert!(open.data().links.is_empty());

    open.undo(&test.db).unwrap();

    assert_eq!(open.data(), &before);
    assert!(same_as_stored(&test.db, &open));

    // Three more undos: the link, the requirement, the input.
    for _ in 0..3 {
        open.undo(&test.db).unwrap();
    }

    assert!(open.data().tiles.is_empty());
    assert!(!open.can_undo());
    assert!(open.undo(&test.db).is_err());
    assert!(same_as_stored(&test.db, &open));
}

#[test]
fn undo_survives_a_number_given_meanwhile_to_another_project() {
    let test = database();
    let document = load(&test.db, b"%PDF numbers");
    let (project, scheme) = project_with_scheme(&test.db, document);
    let mut open = opened(&test.db, project);

    let tile = open
        .change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Input, "Sample")))
        .unwrap();
    open.change(|open| tiles::delete::tile(&test.db, open, tile)).unwrap();

    // Another project takes the freed number.
    let other = projects::create::project(&test.db, &project_fields("Q", vec![document])).unwrap();
    let mut other_open = opened(&test.db, other);
    let other_scheme =
        schemes::create::scheme(&test.db, &mut other_open, &crate::test_projects::test_support::scheme_fields("1", "A"))
            .unwrap();
    let taken = tiles::create::tile(&test.db, &mut other_open, other_scheme, &TileFields::new(TileKind::Input, "Q"))
        .unwrap();
    assert_eq!(taken, tile, "SQLite gives the freed number again");

    // The tile comes back with a new number; memory and database agree.
    open.undo(&test.db).unwrap();

    assert_eq!(open.data().tiles.len(), 1);
    assert_ne!(open.data().tiles[0].id, tile);
    assert_eq!(open.data().tiles[0].fields.title, "Sample");
    assert!(same_as_stored(&test.db, &open));
    assert!(same_as_stored(&test.db, &other_open));
}

#[test]
fn a_long_work_is_one_version_and_deleting_a_scheme_forgets_them() {
    let test = database();
    let document = load(&test.db, b"%PDF long");
    let (project, scheme) = project_with_scheme(&test.db, document);
    let mut open = opened(&test.db, project);

    open.remember();
    tiles::create::tile(&test.db, &mut open, scheme, &TileFields::new(TileKind::Input, "One")).unwrap();
    tiles::create::tile(&test.db, &mut open, scheme, &TileFields::new(TileKind::Input, "Two")).unwrap();

    open.undo(&test.db).unwrap();
    assert!(open.data().tiles.is_empty());
    assert!(!open.can_undo());

    open.change(|open| tiles::create::tile(&test.db, open, scheme, &TileFields::new(TileKind::Input, "Three"))).unwrap();
    assert!(open.can_undo());

    schemes::delete::scheme(&test.db, &mut open, scheme).unwrap();
    assert!(!open.can_undo());
    assert!(open.data().schemes.is_empty() && open.data().tiles.is_empty());
    assert!(same_as_stored(&test.db, &open));
}

#[test]
fn a_workspace_opens_a_project_once_and_closes_it() {
    let test = database();
    let document = load(&test.db, b"%PDF open");
    let (project, _) = project_with_scheme(&test.db, document);
    let mut workspace = Workspace::default();

    workspace.open(&test.db, project).unwrap().remember();
    assert!(workspace.open(&test.db, project).unwrap().can_undo(), "the same object, not read again");

    workspace.close(project);
    assert!(workspace.get(project).is_none());
    assert!(workspace.open(&test.db, 9_999).is_err());
}
