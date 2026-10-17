//! Unit tests of `import.rs`.

use super::*;

use crate::evaluation::exchange;
use crate::evaluation::scheme::{TileFields, TileKind};
use crate::evaluation::schemes;
use crate::evaluation::test_support::{database, scheme_fields};
use crate::product_evaluation::journal;
use crate::product_evaluation::model::JournalAction;
use crate::product_evaluation::test_support::{fields, ready_project};

#[test]
fn a_ready_project_becomes_a_frozen_copy() {
    let test = database();
    let ready = ready_project(&test.db);
    let imported = from_project(&test.db, ready.project, &fields("Meter X")).unwrap();
    assert_eq!(imported.skipped, Skipped::default());

    let data = projects::data(&test.db, imported.id).unwrap().unwrap();
    assert_eq!(data.documents, vec![ready.document]);
    assert_eq!(data.schemes.len(), 1);
    assert_eq!((data.schemes[0].project, data.schemes[0].fields.name.as_str()), (imported.id, "ADV"));
    assert_eq!(data.tiles.len(), 3);
    assert_eq!(data.links.len(), 2);
    assert_eq!(data.evaluation.origin.author, "Head of the laboratory");

    // New numbers of its own, the same content.
    let requirement = data.tiles.iter().find(|tile| tile.fields.kind == TileKind::Requirement).unwrap();
    let original = schemes::tile(&test.db, ready.requirement).unwrap().unwrap();
    assert_eq!(requirement.fields, original.fields);
    assert_eq!(requirement.scheme, data.schemes[0].id);
    assert!(requirement.fields.sources[0].found);

    // The test project changes later; the evaluation does not.
    schemes::delete_tile(&test.db, ready.output).unwrap();
    test_projects::delete(&test.db, ready.project).unwrap();
    let again = projects::data(&test.db, imported.id).unwrap().unwrap();
    assert_eq!(again.tiles.len(), 3);
    assert_eq!(again.links.len(), 2);

    // The first entry of the journal.
    let entries = journal::of(&test.db, imported.id).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, JournalAction::Imported);
    assert!(entries[0].detail.contains("\"P\""), "{}", entries[0].detail);
}

#[test]
fn a_project_not_marked_ready_is_refused() {
    let test = database();
    let ready = ready_project(&test.db);
    test_projects::set_ready(&test.db, ready.project, false).unwrap();

    let refused = from_project(&test.db, ready.project, &fields("Meter X"));
    assert!(matches!(refused, Err(EvaluationError::Invalid("project", _))));
    assert!(projects::list(&test.db).unwrap().is_empty());

    assert!(matches!(from_project(&test.db, 9_999, &fields("Meter X")), Err(EvaluationError::NotFound(_))));
}

#[test]
fn proposals_never_accepted_are_left_out() {
    let test = database();
    let ready = ready_project(&test.db);

    // A proposed tile linked to the requirement.
    let mut proposed = TileFields::new(TileKind::Input, "Proposed input");
    proposed.state = ProposalState::Proposed;
    let proposed = schemes::add_tile(&test.db, ready.scheme, &proposed).unwrap();
    schemes::add_link(&test.db, proposed, ready.requirement).unwrap();

    // A proposed scheme with an accepted tile in it.
    let mut chapter = scheme_fields("6", "ATE");
    chapter.state = ProposalState::Proposed;
    let chapter = schemes::create(&test.db, ready.project, &chapter).unwrap();
    schemes::add_tile(&test.db, chapter, &TileFields::new(TileKind::Requirement, "Independent testing")).unwrap();

    let imported = from_project(&test.db, ready.project, &fields("Meter X")).unwrap();
    assert_eq!(
        imported.skipped,
        Skipped {
            schemes: 1,
            tiles: 2,
            links: 1
        }
    );

    let data = projects::data(&test.db, imported.id).unwrap().unwrap();
    assert_eq!((data.schemes.len(), data.tiles.len(), data.links.len()), (1, 3, 2));
    assert!(data.tiles.iter().all(|tile| tile.fields.state == ProposalState::Accepted));

    let detail = &journal::of(&test.db, imported.id).unwrap()[0].detail;
    assert!(detail.contains("1 schemes, 2 tiles, 1 links"), "{detail}");
}

#[test]
fn the_file_of_a_project_is_imported_the_same_way() {
    let test = database();
    let ready = ready_project(&test.db);
    let file = exchange::export(&test.db, ready.project, false).unwrap().unwrap();
    let read = exchange::read(&file).unwrap();

    let imported = from_exchange(&test.db, &read, &[ready.document], &fields("From the file")).unwrap();
    let from_file = projects::data(&test.db, imported.id).unwrap().unwrap();
    let direct = projects::data(&test.db, from_project(&test.db, ready.project, &fields("Direct")).unwrap().id)
        .unwrap()
        .unwrap();

    let content = |data: &crate::product_evaluation::model::EvaluationData| {
        data.tiles.iter().map(|tile| tile.fields.clone()).collect::<Vec<_>>()
    };
    assert_eq!(content(&from_file), content(&direct));
    assert_eq!(from_file.links.len(), 2);
    assert_eq!(from_file.documents, vec![ready.document]);

    // A document of the file that was not found is refused.
    let missing = from_exchange(&test.db, &read, &[], &fields("Missing"));
    assert!(matches!(missing, Err(EvaluationError::NotFound("document"))));
}

#[test]
fn a_file_changed_by_hand_is_checked_by_the_rules() {
    let test = database();
    let ready = ready_project(&test.db);
    let file = exchange::export(&test.db, ready.project, false).unwrap().unwrap();
    let mut read = exchange::read(&file).unwrap();

    // A link output → input is not allowed.
    let output = read.tiles.iter().find(|tile| tile.fields.kind == TileKind::Output).unwrap().id;
    let input = read.tiles.iter().find(|tile| tile.fields.kind == TileKind::Input).unwrap().id;
    read.links.push(Link { id: 99, from: output, to: input });

    let refused = from_exchange(&test.db, &read, &[ready.document], &fields("Broken"));
    assert!(matches!(refused, Err(EvaluationError::Scheme(SchemeError::LinkNotAllowed(..)))));
    assert!(projects::list(&test.db).unwrap().is_empty());
}
