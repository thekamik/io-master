//! The journal of the questions to the AI (P3 of `architecture.md`:
//! everything is traceable), in the database of an evaluator: every
//! question of every task and role, with what was sent and what came
//! back. **Protection: encrypted by SQLCipher** with the key of the
//! database of the evaluator (`db/encryption/`). The API key is never
//! in it: only the body of a request is kept, and the key travels only
//! in a header (`ai/api/connection.rs`).
//!
//! ```text
//! ai_journal   one row per question: time, task, role, connection,
//!              model, the request, the answer, how it ended, tokens
//! ```
//!
//! **Only added to.** No file of this folder changes or removes an
//! entry, and the database refuses it as well (two triggers): the
//! journal goes only with the whole database. How long entries are
//! kept is to be decided later (a reminder of `docs/todo.md`).

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
/// The task, role and connection are kept by name, as text: the record
/// stays as it was when the names change.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_journal (
        id              INTEGER PRIMARY KEY,
        time            INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER)),
        task            TEXT NOT NULL,
        role            TEXT NOT NULL,
        connection      TEXT NOT NULL,
        model           TEXT NOT NULL,
        request         TEXT NOT NULL,
        answer          TEXT NOT NULL,
        ending          TEXT NOT NULL,
        note            TEXT,
        input_tokens    INTEGER NOT NULL,
        output_tokens   INTEGER NOT NULL
    );

    CREATE TRIGGER IF NOT EXISTS ai_journal_unchanged BEFORE UPDATE ON ai_journal
    BEGIN
        SELECT RAISE(ABORT, 'an entry of the journal of the AI is never changed');
    END;

    CREATE TRIGGER IF NOT EXISTS ai_journal_kept BEFORE DELETE ON ai_journal
    BEGIN
        SELECT RAISE(ABORT, 'an entry of the journal of the AI is never removed');
    END;
";

#[cfg(test)]
mod tests {
    //! Tests of the triggers of the journal: an entry is never changed nor
    //! removed.

    use crate::db::ai::journal::{create, read};
    use crate::test_support::db::journal_entry;
    use crate::test_support::standards::database;

    #[test]
    fn the_database_refuses_to_change_or_remove_an_entry() {
        let test = database();
        create::entry(&test.db, &journal_entry("chat")).unwrap();

        let changed = test.db.connection().execute("UPDATE ai_journal SET answer = 'other'", []);
        assert!(changed.unwrap_err().to_string().contains("never changed"));

        let removed = test.db.connection().execute("DELETE FROM ai_journal", []);
        assert!(removed.unwrap_err().to_string().contains("never removed"));

        assert_eq!(read::last(&test.db, 10).unwrap()[0].answer, "Hello");
    }
}
