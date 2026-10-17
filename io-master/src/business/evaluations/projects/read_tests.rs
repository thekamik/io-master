//! Unit tests of `projects/` (attached to `read.rs`) and of the open
//! evaluation in memory (`workspace.rs`).

use super::*;

use crate::business::evaluations::error::EvaluationError;
use crate::business::evaluations::journal;
use crate::business::evaluations::projects::{create as import, delete as remove};
use crate::business::evaluations::test_support::single as results;
use crate::business::evaluations::test_support::{fields, ready_project};
use crate::business::evaluations::workspace::Workspace;
use crate::business::standards;
use crate::business::standards::error::DocumentError;
use crate::business::test_projects::test_support::database;
use crate::model::evaluations::ResultField;
use crate::model::test_projects::TileKind;

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
    assert!(matches!(standards::delete::delete(&test.db, ready.document), Err(DocumentError::InUse)));

    let mut workspace = Workspace::default();
    workspace.open(&test.db, id).unwrap();

    assert!(remove::evaluation(&test.db, &mut workspace, id).unwrap());
    assert!(get(&test.db, id).unwrap().is_none());
    assert!(crate::db::evaluations::read::tile(&test.db, requirement).unwrap().is_none());
    assert!(journal::read::of(&test.db, id).unwrap().is_empty());
    // Closed in memory as well.
    assert!(matches!(workspace.open(&test.db, id), Err(EvaluationError::NotFound(_))));

    assert!(!remove::evaluation(&test.db, &mut workspace, id).unwrap());
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
fn an_open_evaluation_holds_its_structure_and_results() {
    let test = database();
    let ready = ready_project(&test.db);
    let id = import::from_project(&test.db, ready.project, &fields("Meter X")).unwrap().id;
    let requirement = data(&test.db, id).unwrap().unwrap().tiles[1].id;
    results::write(&test.db, requirement, ResultField::Findings, "None.").unwrap();

    let mut workspace = Workspace::default();
    let open = workspace.open(&test.db, id).unwrap();

    assert_eq!(open.data(), &data(&test.db, id).unwrap().unwrap());
    assert_eq!(open.results().len(), 1);
    assert!(open.proposals().is_empty());
    assert!(workspace.open(&test.db, 9_999).is_err());
}
