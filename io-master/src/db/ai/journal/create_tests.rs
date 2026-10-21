//! Unit tests of `db/ai/journal/` (attached to `create.rs`).

use super::*;

use crate::db::ai::journal::read;
use crate::test_support::standards::database;

fn sample(task: &str) -> Entry {
    Entry {
        id: 0,
        time: 0,
        task: task.to_owned(),
        role: "consultation".to_owned(),
        connection: "Local".to_owned(),
        model: "gemma4".to_owned(),
        request: "{\"messages\": []}".to_owned(),
        answer: "Hello".to_owned(),
        ending: "complete".to_owned(),
        note: None,
        input_tokens: 12,
        output_tokens: 3,
    }
}

#[test]
fn entries_are_added_and_read_newest_first() {
    let test = database();

    entry(&test.db, &sample("chat")).unwrap();
    entry(&test.db, &sample("chapter")).unwrap();

    let entries = read::last(&test.db, 10).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].task, "chapter");
    assert_eq!(entries[1], Entry { id: entries[1].id, time: entries[1].time, ..sample("chat") });
    assert!(entries[0].time > 1_700_000_000);
    assert_eq!(read::last(&test.db, 1).unwrap().len(), 1);
}

#[test]
fn the_database_refuses_to_change_or_remove_an_entry() {
    let test = database();
    entry(&test.db, &sample("chat")).unwrap();

    let changed = test.db.connection().execute("UPDATE ai_journal SET answer = 'other'", []);
    assert!(changed.unwrap_err().to_string().contains("never changed"));

    let removed = test.db.connection().execute("DELETE FROM ai_journal", []);
    assert!(removed.unwrap_err().to_string().contains("never removed"));

    assert_eq!(read::last(&test.db, 10).unwrap()[0].answer, "Hello");
}
