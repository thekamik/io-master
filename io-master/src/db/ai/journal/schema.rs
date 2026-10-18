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
//! kept is to be decided later (`docs/todo.md`, step 38).

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
