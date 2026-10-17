//! Unit tests of the journal (`create.rs`, `read.rs`, `rows.rs`), and
//! of the triggers of `db/evaluations/schema.rs` that keep it from being
//! changed.

use super::*;

use crate::db::error::DbError;
use crate::db::evaluations::journal::read;
use crate::db::user::database::UserDb;
use crate::test_projects::test_support::database;

fn step<'a>(actor: &'a Actor, action: JournalAction, detail: &'a str) -> JournalStep<'a> {
    JournalStep { actor, action, tile: Some(7), field: Some(ResultField::Analysis), detail }
}

/// An evaluation row the journal can belong to, without a structure.
fn evaluation(db: &UserDb) -> i64 {
    db.connection_for_tests()
        .execute(
            "INSERT INTO evaluations (name, product, product_version, manufacturer, evaluator, source_name, \
             source_version, source_author, source_description) VALUES ('E', 'P', '', '', '', '', '', '', '')",
            [],
        )
        .unwrap();

    db.connection_for_tests().last_insert_rowid()
}

#[test]
fn entries_are_read_back_in_their_order() {
    let test = database();
    let id = evaluation(&test.db);
    let model = Actor::Model("gemma4".to_owned());

    entry(test.db.connection_for_tests(), id, &step(&model, JournalAction::Proposed, "The text of the AI")).unwrap();
    entry(test.db.connection_for_tests(), id, &step(&Actor::Evaluator, JournalAction::Accepted, "The text of the AI"))
        .unwrap();

    let entries = read::entries(&test.db, id).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].actor, model);
    assert_eq!(entries[0].action, JournalAction::Proposed);
    assert_eq!(entries[0].tile, Some(7));
    assert_eq!(entries[0].field, Some(ResultField::Analysis));
    assert_eq!(entries[1].actor, Actor::Evaluator);
    assert!(entries[1].time > 1_700_000_000);

    // The journal of another evaluation is its own.
    assert!(read::entries(&test.db, id + 1).unwrap().is_empty());
}

#[test]
fn the_database_refuses_to_change_or_remove_an_entry() {
    let test = database();
    let id = evaluation(&test.db);

    entry(test.db.connection_for_tests(), id, &step(&Actor::Evaluator, JournalAction::Written, "first")).unwrap();

    let changed = test.db.connection_for_tests().execute("UPDATE evaluation_journal SET detail = 'other'", []);
    assert!(changed.unwrap_err().to_string().contains("never changed"));

    let removed = test.db.connection_for_tests().execute("DELETE FROM evaluation_journal", []);
    assert!(removed.unwrap_err().to_string().contains("whole evaluation"));

    assert_eq!(read::entries(&test.db, id).unwrap()[0].detail, "first");

    // Once the evaluation is gone, its journal can go too.
    test.db.connection_for_tests().execute("DELETE FROM evaluations WHERE id = ?1", [id]).unwrap();
    test.db.connection_for_tests().execute("DELETE FROM evaluation_journal WHERE evaluation_id = ?1", [id]).unwrap();
    assert!(read::entries(&test.db, id).unwrap().is_empty());
}

#[test]
fn an_entry_changed_outside_the_program_is_an_error_not_a_guess() {
    let test = database();
    let id = evaluation(&test.db);

    entry(test.db.connection_for_tests(), id, &step(&Actor::Evaluator, JournalAction::Written, "first")).unwrap();

    // The trigger keeps the program from changing it; a file edited by
    // hand is simulated without it.
    test.db
        .connection_for_tests()
        .execute_batch(
            "DROP TRIGGER evaluation_journal_unchanged; UPDATE evaluation_journal SET action = 'approved_by_magic';",
        )
        .unwrap();

    assert!(matches!(read::entries(&test.db, id), Err(DbError::Corrupt(corrupt)) if corrupt.column == "action"));
}
