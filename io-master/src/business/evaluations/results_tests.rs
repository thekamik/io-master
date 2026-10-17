//! Unit tests of `results/` (attached to `update.rs`; they cover the
//! whole folder), through `test_support::single`.

use crate::business::evaluations::error::EvaluationError;
use crate::business::evaluations::journal;
use crate::business::evaluations::projects;
use crate::business::evaluations::test_support::single::{accept, correct, of, proposals, propose, reject, write};
use crate::business::evaluations::test_support::{fields, ready_project};
use crate::business::test_projects::test_support::database;
use crate::db::user::database::UserDb;
use crate::model::evaluations::{Actor, EvaluationData, JournalAction, JournalEntry, ResultField, ResultOrigin};
use crate::model::test_projects::TileKind;
use crate::model::text::{Entity, Field};

/// A database with one imported evaluation; returns it and the
/// number of its requirement.
fn evaluation() -> (crate::business::test_projects::test_support::TestDb, EvaluationData, i64) {
    let test = database();
    let ready = ready_project(&test.db);
    let id = projects::from_project(&test.db, ready.project, &fields("Meter X")).unwrap().id;
    let data = projects::data(&test.db, id).unwrap().unwrap();
    let requirement = data.tiles.iter().find(|tile| tile.fields.kind == TileKind::Requirement).unwrap().id;

    (test, data, requirement)
}

fn journal_of(db: &UserDb, data: &EvaluationData) -> Vec<JournalEntry> {
    journal::list(db, data.evaluation.id).unwrap()
}

#[test]
fn a_proposal_is_no_result_until_it_is_accepted() {
    let (test, data, requirement) = evaluation();

    let id = propose(&test.db, requirement, ResultField::Analysis, "The interfaces are described.", "gemma4").unwrap();
    assert!(of(&test.db, requirement).unwrap().is_empty());

    let waiting = proposals(&test.db, requirement).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!((waiting[0].field, waiting[0].model.as_str()), (ResultField::Analysis, "gemma4"));

    accept(&test.db, id).unwrap();

    let results = of(&test.db, requirement).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].text, "The interfaces are described.");
    assert_eq!(results[0].origin, ResultOrigin::Ai);
    assert!(proposals(&test.db, requirement).unwrap().is_empty());

    let entries = journal_of(&test.db, &data);
    assert_eq!(entries[1].action, JournalAction::Proposed);
    assert_eq!(entries[1].actor, Actor::Model("gemma4".to_owned()));
    assert_eq!(entries[2].action, JournalAction::Accepted);
    assert_eq!(entries[2].actor, Actor::Evaluator);

    // Done once only.
    assert!(matches!(accept(&test.db, id), Err(EvaluationError::NotFound(Entity::Proposal))));
}

#[test]
fn a_corrected_proposal_keeps_both_texts_in_the_journal() {
    let (test, data, requirement) = evaluation();

    let id = propose(&test.db, requirement, ResultField::Assessment, "Pass.", "gemma4").unwrap();
    correct(&test.db, id, "Pass, with one open point.").unwrap();

    let results = of(&test.db, requirement).unwrap();
    assert_eq!(results[0].text, "Pass, with one open point.");
    assert_eq!(results[0].origin, ResultOrigin::AiCorrected);

    let last = journal_of(&test.db, &data).pop().unwrap();
    assert_eq!(last.action, JournalAction::Corrected);
    assert!(last.detail.contains("Pass, with one open point.") && last.detail.contains("\nPass."), "{}", last.detail);

    // A "correction" without a change is an acceptance.
    let id = propose(&test.db, requirement, ResultField::Findings, "None.", "gemma4").unwrap();
    correct(&test.db, id, "None.").unwrap();
    assert_eq!(of(&test.db, requirement).unwrap()[0].origin, ResultOrigin::Ai);

    // An empty correction is refused: reject instead.
    let id = propose(&test.db, requirement, ResultField::Analysis, "Text.", "gemma4").unwrap();
    assert!(matches!(correct(&test.db, id, "  "), Err(EvaluationError::Invalid(Field::Text, _))));
}

#[test]
fn a_new_proposal_does_not_touch_an_accepted_result() {
    let (test, data, requirement) = evaluation();

    write(&test.db, requirement, ResultField::Vulnerabilities, "A weak default password.").unwrap();
    let id = propose(&test.db, requirement, ResultField::Vulnerabilities, "None.", "gemma4").unwrap();

    assert_eq!(of(&test.db, requirement).unwrap()[0].text, "A weak default password.");

    reject(&test.db, id).unwrap();
    assert_eq!(of(&test.db, requirement).unwrap()[0].text, "A weak default password.");
    assert!(proposals(&test.db, requirement).unwrap().is_empty());

    let last = journal_of(&test.db, &data).pop().unwrap();
    assert_eq!((last.action, last.detail.as_str()), (JournalAction::Rejected, "None."));
}

#[test]
fn the_evaluator_writes_changes_and_removes_a_result() {
    let (test, data, requirement) = evaluation();

    write(&test.db, requirement, ResultField::Findings, "First.").unwrap();
    write(&test.db, requirement, ResultField::Findings, "Second.").unwrap();
    // The same text again: nothing to record.
    write(&test.db, requirement, ResultField::Findings, "Second.").unwrap();

    let results = of(&test.db, requirement).unwrap();
    assert_eq!((results[0].text.as_str(), results[0].origin), ("Second.", ResultOrigin::Evaluator));

    let entries = journal_of(&test.db, &data);
    assert_eq!(entries.len(), 3);
    assert!(entries[2].detail.contains("Second.") && entries[2].detail.contains("First."));

    write(&test.db, requirement, ResultField::Findings, "").unwrap();
    assert!(of(&test.db, requirement).unwrap().is_empty());
    assert!(journal_of(&test.db, &data).pop().unwrap().detail.contains("removed"));
}

#[test]
fn results_are_in_the_order_of_the_report() {
    let (test, _, requirement) = evaluation();

    for field in ResultField::ALL.into_iter().rev() {
        write(&test.db, requirement, field, field.title()).unwrap();
    }

    let fields: Vec<ResultField> = of(&test.db, requirement).unwrap().iter().map(|result| result.field).collect();
    assert_eq!(fields, ResultField::ALL);
}

#[test]
fn results_belong_to_requirements_only() {
    let (test, data, _) = evaluation();
    let input = data.tiles.iter().find(|tile| tile.fields.kind == TileKind::Input).unwrap().id;

    assert!(matches!(write(&test.db, input, ResultField::Findings, "x"), Err(EvaluationError::NotARequirement)));
    assert!(matches!(
        propose(&test.db, input, ResultField::Findings, "x", "gemma4"),
        Err(EvaluationError::NotARequirement)
    ));
    assert!(matches!(write(&test.db, 9_999, ResultField::Findings, "x"), Err(EvaluationError::NotFound(Entity::Tile))));
}

#[test]
fn texts_are_checked() {
    let (test, _, requirement) = evaluation();

    assert!(matches!(
        propose(&test.db, requirement, ResultField::Findings, " ", "gemma4"),
        Err(EvaluationError::Invalid(Field::Text, _))
    ));
    assert!(matches!(
        write(&test.db, requirement, ResultField::Findings, "bell\u{7}"),
        Err(EvaluationError::Invalid(Field::Text, _))
    ));
    assert!(write(&test.db, requirement, ResultField::Findings, "line 1\nline 2").is_ok());
}
