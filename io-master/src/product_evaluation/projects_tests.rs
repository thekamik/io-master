//! Unit tests of `projects.rs`; the copy is made by `import.rs`.

use super::*;

use crate::evaluation::documents::{self, DocumentError};
use crate::evaluation::scheme::TileKind;
use crate::evaluation::test_support::database;
use crate::product_evaluation::import;
use crate::product_evaluation::journal;
use crate::product_evaluation::model::ResultField;
use crate::product_evaluation::results;
use crate::product_evaluation::test_support::{fields, ready_project};

#[test]
fn an_evaluation_is_listed_read_and_deleted_with_everything_in_it() {
    let test = database();
    let ready = ready_project(&test.db);
    let id = import::from_project(&test.db, ready.project, &fields("Meter X")).unwrap().id;

    let listed = list(&test.db).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].fields, fields("Meter X"));
    assert_eq!(listed[0].origin.name, "P");

    let data = data(&test.db, id).unwrap().unwrap();
    let requirement = data.tiles.iter().find(|tile| tile.fields.kind == TileKind::Requirement).unwrap().id;
    results::write(&test.db, requirement, ResultField::Findings, "None.").unwrap();
    results::propose(&test.db, requirement, ResultField::Analysis, "Proposed.", "gemma4").unwrap();

    // While the evaluation uses the standard, the standard stays.
    assert!(matches!(documents::delete(&test.db, ready.document), Err(DocumentError::InUse)));

    assert!(delete(&test.db, id).unwrap());
    assert!(get(&test.db, id).unwrap().is_none());
    assert!(tile(&test.db, requirement).unwrap().is_none());
    assert!(journal::of(&test.db, id).unwrap().is_empty());

    assert!(!delete(&test.db, id).unwrap());
}

#[test]
fn names_are_unique() {
    let test = database();
    let ready = ready_project(&test.db);
    import::from_project(&test.db, ready.project, &fields("Meter X")).unwrap();

    let again = import::from_project(&test.db, ready.project, &fields("meter x"));
    assert!(matches!(again, Err(EvaluationError::NameTaken)));
    assert_eq!(list(&test.db).unwrap().len(), 1);
}

#[test]
fn a_tile_names_its_evaluation() {
    let test = database();
    let ready = ready_project(&test.db);
    let id = import::from_project(&test.db, ready.project, &fields("Meter X")).unwrap().id;
    let data = data(&test.db, id).unwrap().unwrap();

    let (evaluation, found) = tile(&test.db, data.tiles[1].id).unwrap().unwrap();
    assert_eq!(evaluation, id);
    assert_eq!(found, data.tiles[1]);
    assert!(tile(&test.db, 9_999).unwrap().is_none());
}
