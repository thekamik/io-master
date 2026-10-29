//! The journal of the AI (`JournalEntry`), in the table `ai_journal` of the
//! database of the user: **create and read only** (user, 2026-10-28). An
//! entry is never changed nor removed, and the database refuses it as well
//! (two triggers), so that what a result of the AI rests on cannot be
//! altered.
//!
//! The integration and the model are kept by their names, the work unit by
//! its id without a link the database checks: an entry stays as it was when
//! they are renamed, or a work unit is taken out of its plan. The agent is
//! a link the database checks (user, 2026-10-29): agents are never removed.
//!
//! ```text
//! ai_journal   id, time, integration, model, agent, work_unit, request, answer, ending,
//!              input_tokens, output_tokens
//! ```

use rusqlite::{Row, named_params};

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::model::ai::journal::{Ending, JournalEntry};

/// The most entries read at once: one page of the journal.
pub const PAGE: usize = 50;

/// The table and its two triggers, made when they are missing
/// (`database.rs`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_journal (
        id            INTEGER PRIMARY KEY,
        time          INTEGER NOT NULL,
        integration   TEXT NOT NULL,
        model         TEXT NOT NULL,
        agent         INTEGER REFERENCES ai_agents(id),
        work_unit     INTEGER,
        request       TEXT NOT NULL,
        answer        TEXT NOT NULL,
        ending        TEXT NOT NULL CHECK (ending IN ('complete', 'cut', 'stopped', 'refused', 'failed')),
        input_tokens  INTEGER NOT NULL,
        output_tokens INTEGER NOT NULL
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

/// The columns every read selects.
const COLUMNS: &str =
    "id, time, integration, model, agent, work_unit, request, answer, ending, input_tokens, output_tokens";

/// The name of an ending in the database.
fn ending_name(ending: Ending) -> &'static str {
    match ending {
        Ending::Complete => "complete",
        Ending::Cut => "cut",
        Ending::Stopped => "stopped",
        Ending::Refused => "refused",
        Ending::Failed => "failed",
    }
}

/// The ending of its name; a name the program never writes is
/// `DbError::Corrupt`.
fn ending_of(name: &str) -> Result<Ending, DbError> {
    match name {
        "complete" => Ok(Ending::Complete),
        "cut" => Ok(Ending::Cut),
        "stopped" => Ok(Ending::Stopped),
        "refused" => Ok(Ending::Refused),
        "failed" => Ok(Ending::Failed),
        _ => Err(DbError::Corrupt { table: "ai_journal", column: "ending", value: name.to_owned() }),
    }
}

/// Writes a new entry and gives its id. A new object has no id yet (`id`
/// 0): the database gives it.
///
/// Called by: nothing yet (the AI layer rewritten on `db/`: every question).
pub fn create_journal_entry(db: &Database, entry: &JournalEntry) -> Result<i64, DbError> {
    ensure!(entry.id == 0, DbError::Inconsistent("a new entry has an id already".to_owned()));
    ensure!(!entry.request.is_empty(), DbError::Inconsistent("an entry without its request".to_owned()));

    let connection = db.connection();

    connection.execute(
        "INSERT INTO ai_journal (time, integration, model, agent, work_unit, request, answer, ending, input_tokens,
                                 output_tokens)
         VALUES (:time, :integration, :model, :agent, :work_unit, :request, :answer, :ending, :input_tokens,
                 :output_tokens)",
        named_params! {
            ":time": entry.time,
            ":integration": entry.integration,
            ":model": entry.model,
            ":agent": entry.agent,
            ":work_unit": entry.work_unit,
            ":request": entry.request,
            ":answer": entry.answer,
            ":ending": ending_name(entry.ending),
            ":input_tokens": entry.input_tokens,
            ":output_tokens": entry.output_tokens,
        },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The model of a row of `ai_journal`.
///
/// Called by: [`read_journal_entry`], [`read_journal_entries`].
fn entry_of(row: &Row<'_>) -> Result<JournalEntry, DbError> {
    let ending: String = row.get("ending")?;

    Ok(JournalEntry {
        id: row.get("id")?,
        time: row.get("time")?,
        integration: row.get("integration")?,
        model: row.get("model")?,
        agent: row.get("agent")?,
        work_unit: row.get("work_unit")?,
        request: row.get("request")?,
        answer: row.get("answer")?,
        ending: ending_of(&ending)?,
        input_tokens: row.get("input_tokens")?,
        output_tokens: row.get("output_tokens")?,
    })
}

/// Reads the entry `id`.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`entry_of`].
pub fn read_journal_entry(db: &Database, id: i64) -> Result<JournalEntry, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM ai_journal WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_journal", id });
    };

    entry_of(row)
}

/// Reads one page of the journal, the newest first: the entries older than
/// `before` (all with `None`), at most [`PAGE`].
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`entry_of`].
pub fn read_journal_entries(db: &Database, before: Option<i64>) -> Result<Vec<JournalEntry>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM ai_journal WHERE :before IS NULL OR id < :before ORDER BY id DESC LIMIT :page"
    ))?;
    let page = i64::try_from(PAGE).map_err(|error| DbError::Inconsistent(error.to_string()))?;
    let mut rows = statement.query(named_params! { ":before": before, ":page": page })?;
    let mut entries = Vec::with_capacity(PAGE);

    for _ in 0..PAGE {
        let Some(row) = rows.next()? else { break };

        entries.push(entry_of(row)?);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    //! Tests of the journal: written, read by pages; never changed nor
    //! removed.

    use super::*;

    use crate::test_support::database::database;

    fn entry(request: &str) -> JournalEntry {
        JournalEntry {
            id: 0,
            time: 1_793_000_000,
            integration: "Ollama".to_owned(),
            model: "llama3.1".to_owned(),
            agent: None,
            work_unit: Some(3),
            request: request.to_owned(),
            answer: "PASS".to_owned(),
            ending: Ending::Complete,
            input_tokens: 120,
            output_tokens: 4,
        }
    }

    #[test]
    fn entries_are_written_and_read_by_pages() {
        let (_directory, db) = database();
        let first = create_journal_entry(&db, &entry("one")).unwrap();
        let second = create_journal_entry(&db, &entry("two")).unwrap();

        assert_eq!(read_journal_entry(&db, first).unwrap(), JournalEntry { id: first, ..entry("one") });
        assert_eq!(read_journal_entries(&db, None).unwrap().len(), 2);
        assert_eq!(read_journal_entries(&db, Some(second)).unwrap(), vec![JournalEntry { id: first, ..entry("one") }]);
    }

    #[test]
    fn the_database_refuses_to_change_or_remove_an_entry() {
        let (_directory, db) = database();
        let id = create_journal_entry(&db, &entry("one")).unwrap();
        let connection = db.connection();

        assert!(connection.execute("UPDATE ai_journal SET answer = 'FAIL' WHERE id = ?1", [id]).is_err());
        assert!(connection.execute("DELETE FROM ai_journal WHERE id = ?1", [id]).is_err());
        assert_eq!(read_journal_entry(&db, id).unwrap().answer, "PASS");
    }

    #[test]
    fn an_entry_names_its_agent_by_its_id_and_only_an_agent_that_exists() {
        let (_directory, db) = database();
        let integration =
            crate::db::ai::integration::create_integration(&db, &crate::test_support::database::integration()).unwrap();
        let agent =
            crate::db::ai::agent::create_agent(&db, &crate::test_support::database::agent(integration)).unwrap();
        let id = create_journal_entry(&db, &JournalEntry { agent: Some(agent), ..entry("one") }).unwrap();

        assert_eq!(read_journal_entry(&db, id).unwrap().agent, Some(agent));
        assert!(matches!(
            create_journal_entry(&db, &JournalEntry { agent: Some(agent + 1), ..entry("two") }),
            Err(DbError::Sql(_))
        ));
    }
}
