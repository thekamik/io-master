//! The journal of the AI (`JournalEntry`), in the table `ai_journal` of the
//! database of the user: **create and read only** (user, 2026-10-28). An
//! entry is never changed nor removed, and the database refuses it as well
//! (two triggers), so that what a result of the AI rests on cannot be
//! altered.
//!
//! The integration and the model are kept by their names, the work unit by
//! its id without a link the database checks: an entry stays as it was when
//! they are renamed, or a work unit is taken out of its plan. The agent is
//! a link the database checks, never empty (user, 2026-10-29): every entry
//! belongs to the journal of its agent, and agents are never removed.
//!
//! ```text
//! ai_journal   id, time, integration, model, agent, work_unit, request, answer, ending,
//!              input_tokens, output_tokens
//! ```

use rusqlite::{Row, named_params};

use crate::db::error::DbError;
use crate::db::storage::user_database::UserDatabase;
use crate::model::ai::journal::{Ending, JournalEntry};

/// The most entries read at once: one page of the journal.
pub const PAGE: usize = 50;

/// The table and its two triggers, made when they are missing
/// (`schema.rs`, `USER_TABLES`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_journal (
        id            INTEGER PRIMARY KEY,
        time          INTEGER NOT NULL,
        integration   TEXT NOT NULL,
        model         TEXT NOT NULL,
        agent         INTEGER NOT NULL REFERENCES ai_agents(id),
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

/// The name of an ending in the column `ending` (`Ending::Stopped` →
/// "stopped").
///
/// Called by: [`create_journal_entry`].
fn ending_name(ending: Ending) -> &'static str {
    match ending {
        Ending::Complete => "complete",
        Ending::Cut => "cut",
        Ending::Stopped => "stopped",
        Ending::Refused => "refused",
        Ending::Failed => "failed",
    }
}

/// The ending of its name in the column `ending` ("stopped" →
/// `Ending::Stopped`); a name the program never writes is
/// `DbError::Corrupt`.
///
/// Called by: [`entry_of`].
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

/// Writes a new entry of the journal of an agent, a question and its answer,
/// and gives its id. An entry is never changed afterwards.
///
/// Input:
/// - `db`: the open database of the user.
/// - `entry`: the new entry, its `id` 0 (the database gives it): the time,
///   the integration and the model by their names, the agent, the work unit
///   (or none), the request, the answer, how it ended, the tokens.
///
/// Output: the id of the entry (57). An error when the request is empty or
/// the agent is not stored (foreign key).
///
/// Called by: `ai/ask.rs`: `write_entry`.
/// Calls: [`ending_name`].
pub fn create_journal_entry(db: &UserDatabase, entry: &JournalEntry) -> Result<i64, DbError> {
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

/// The model of one row of `ai_journal`.
///
/// Called by: [`read_journal_entry`], [`read_journal_entries`],
/// [`read_latest_journal_entry`].
/// Calls: [`ending_of`].
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

/// Reads the entry `id`, with its whole request and answer.
///
/// Input: `db`, the open database of the user; `id`, the id of the entry (57).
///
/// Output: the entry; `DbError::NoRow` when there is none.
///
/// Called by: `panels/views/agents/journal.rs`: `agent_journal_get`.
/// Calls: [`entry_of`].
pub fn read_journal_entry(db: &UserDatabase, id: i64) -> Result<JournalEntry, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!("SELECT {COLUMNS} FROM ai_journal WHERE id = :id"))?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "ai_journal", id });
    };

    entry_of(row)
}

/// Reads one page of the journal of an agent, the newest first.
///
/// Input:
/// - `db`: the open database of the user.
/// - `agent`: the id of the agent.
/// - `before`: the id of the oldest entry of the page shown, to read the
///   next page (the entries older than it); `None` for the first page.
///
/// Output: at most [`PAGE`] entries; none past the last page.
///
/// Called by: `panels/views/agents/journal.rs`: `agent_journal_list`.
/// Calls: [`entry_of`].
pub fn read_journal_entries(db: &UserDatabase, agent: i64, before: Option<i64>) -> Result<Vec<JournalEntry>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare(&format!(
        "SELECT {COLUMNS} FROM ai_journal WHERE agent = :agent AND (:before IS NULL OR id < :before)
         ORDER BY id DESC LIMIT :page"
    ))?;
    let page = i64::try_from(PAGE).map_err(|error| DbError::Inconsistent(error.to_string()))?;
    let mut rows = statement.query(named_params! { ":agent": agent, ":before": before, ":page": page })?;
    let mut entries = Vec::with_capacity(PAGE);

    for _ in 0..PAGE {
        let Some(row) = rows.next()? else { break };

        entries.push(entry_of(row)?);
    }

    Ok(entries)
}

/// Reads the newest entry about a work unit, of any agent: the last question
/// of the AI about it and its answer.
///
/// Input: `db`, the open database of the user; `work_unit`, the id of the
/// work unit.
///
/// Output: the entry; `None` when the AI was never asked about it.
///
/// Called by: `panels/views/evaluation/work_units.rs`: `unit_view`.
/// Calls: [`entry_of`].
pub fn read_latest_journal_entry(db: &UserDatabase, work_unit: i64) -> Result<Option<JournalEntry>, DbError> {
    let connection = db.connection();
    let mut statement = connection
        .prepare(&format!("SELECT {COLUMNS} FROM ai_journal WHERE work_unit = :work_unit ORDER BY id DESC LIMIT 1"))?;
    let mut rows = statement.query(named_params! { ":work_unit": work_unit })?;

    Ok(match rows.next()? {
        Some(row) => Some(entry_of(row)?),
        None => None,
    })
}

#[cfg(test)]
mod tests {
    //! Tests of the journal: written, read by pages and by work unit; never
    //! changed nor removed.

    use super::*;

    use crate::db::models::ai::agent::create_agent;
    use crate::db::models::ai::integration::create_integration;
    use crate::model::ai::agent::Agent;
    use crate::model::ai::integration::Integration;
    use crate::test_support::TempDir;
    use crate::test_support::database::{agent, database, integration};

    /// A new database with one agent (its id), which every entry names.
    fn database_with_agent() -> (TempDir, UserDatabase, i64) {
        let (directory, db) = database();
        let integration = create_integration(&db, &integration()).unwrap();
        let agent = create_agent(&db, &agent(integration)).unwrap();

        (directory, db, agent)
    }

    fn entry(agent: i64, request: &str) -> JournalEntry {
        JournalEntry {
            id: 0,
            time: 1_793_000_000,
            integration: "Ollama".to_owned(),
            model: "llama3.1".to_owned(),
            agent,
            work_unit: Some(3),
            request: request.to_owned(),
            answer: "PASS".to_owned(),
            ending: Ending::Complete,
            input_tokens: 120,
            output_tokens: 4,
        }
    }

    #[test]
    fn the_entries_of_an_agent_are_written_and_read_by_pages() {
        let (_directory, db, agent) = database_with_agent();
        let integration = create_integration(&db, &Integration { name: "Other".to_owned(), ..integration() }).unwrap();
        let other =
            create_agent(&db, &Agent { name: "Other".to_owned(), ..crate::test_support::database::agent(integration) })
                .unwrap();
        let first = create_journal_entry(&db, &entry(agent, "one")).unwrap();
        let second = create_journal_entry(&db, &entry(agent, "two")).unwrap();

        create_journal_entry(&db, &entry(other, "of another agent")).unwrap();

        assert_eq!(read_journal_entry(&db, first).unwrap(), JournalEntry { id: first, ..entry(agent, "one") });
        assert_eq!(read_journal_entries(&db, agent, None).unwrap().len(), 2);
        assert_eq!(
            read_journal_entries(&db, agent, Some(second)).unwrap(),
            vec![JournalEntry { id: first, ..entry(agent, "one") }]
        );
        assert_eq!(read_journal_entries(&db, other, None).unwrap().len(), 1);

        let latest = read_latest_journal_entry(&db, 3).unwrap().unwrap();

        assert_eq!((latest.request.as_str(), latest.agent), ("of another agent", other));
        assert_eq!(read_latest_journal_entry(&db, 4).unwrap(), None);
    }

    #[test]
    fn the_database_refuses_to_change_or_remove_an_entry() {
        let (_directory, db, agent) = database_with_agent();
        let id = create_journal_entry(&db, &entry(agent, "one")).unwrap();
        let connection = db.connection();

        assert!(connection.execute("UPDATE ai_journal SET answer = 'FAIL' WHERE id = ?1", [id]).is_err());
        assert!(connection.execute("DELETE FROM ai_journal WHERE id = ?1", [id]).is_err());
        assert_eq!(read_journal_entry(&db, id).unwrap().answer, "PASS");
    }

    #[test]
    fn an_entry_names_only_an_agent_that_exists() {
        let (_directory, db, agent) = database_with_agent();

        assert!(matches!(create_journal_entry(&db, &entry(agent + 1, "two")), Err(DbError::Sql(_))));
    }
}
