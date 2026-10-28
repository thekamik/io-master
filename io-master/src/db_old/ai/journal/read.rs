//! Reading the journal of the AI: a page of its entries, the newest first,
//! by task and words (step 58: the page of the journal, P3), and one entry
//! whole.

use rusqlite::{OptionalExtension, named_params};

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;

use super::rows::{self, Entry, EntrySummary};

/// The entries of a page of the list.
pub const PAGE_SIZE: usize = 50;

/// The most tasks named in the filter of the list.
const MAX_TASKS: usize = 100;

/// The columns of the list: all but the request and the answer.
const SUMMARY_COLUMNS: &str = "id, time, task, role, connection, model, ending, input_tokens, output_tokens";

/// The condition of the list: of task `:task` (all when NULL), holding the
/// words `:text` in the request or the answer (any case; all when empty).
const FILTER: &str = "(:task IS NULL OR task = :task)
     AND (:text = '' OR instr(lower(request), lower(:text)) > 0 OR instr(lower(answer), lower(:text)) > 0)";

/// A number as a parameter of SQL.
fn sql_number(value: usize) -> Result<i64, DbError> {
    i64::try_from(value).map_err(|error| DbError::Inconsistent(error.to_string()))
}

/// Page `page` (from 0) of the entries of task `task` (all: `None`) that
/// hold `text`, the newest first, at most `PAGE_SIZE`; and how many entries
/// the filter keeps in all.
///
/// Called by: `ai_journal_get` (`views/settings/settings.rs`).
pub fn summaries(
    db: &UserDb,
    task: Option<&str>,
    text: &str,
    page: usize,
) -> Result<(Vec<EntrySummary>, usize), DbError> {
    let connection = db.connection();
    let offset = sql_number(page.saturating_mul(PAGE_SIZE))?;
    let total: i64 = connection.query_row(
        &format!("SELECT COUNT(*) FROM ai_journal WHERE {FILTER}"),
        named_params! { ":task": task, ":text": text },
        |row| row.get(0),
    )?;
    let mut statement = connection.prepare(&format!(
        "SELECT {SUMMARY_COLUMNS} FROM ai_journal WHERE {FILTER} ORDER BY id DESC LIMIT :limit OFFSET :offset"
    ))?;
    let parameters =
        named_params! { ":task": task, ":text": text, ":limit": sql_number(PAGE_SIZE)?, ":offset": offset };
    let mut found = Vec::with_capacity(PAGE_SIZE);

    for row in statement.query_map(parameters, |row| Ok(rows::summary(row)))? {
        found.push(row??);
    }

    let total = usize::try_from(total).map_err(|error| DbError::Inconsistent(error.to_string()))?;

    Ok((found, total))
}

/// Entry `id` whole; `None` when there is no such.
///
/// Called by: `ai_journal_entry` (`views/settings/settings.rs`).
pub fn entry(db: &UserDb, id: i64) -> Result<Option<Entry>, DbError> {
    db.connection()
        .query_row("SELECT * FROM ai_journal WHERE id = ?1", [id], |row| Ok(rows::entry(row)))
        .optional()?
        .transpose()
}

/// The tasks the journal has entries of, by name, for the filter of the list.
///
/// Called by: `ai_journal_get` (`views/settings/settings.rs`).
pub fn tasks(db: &UserDb) -> Result<Vec<String>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT DISTINCT task FROM ai_journal ORDER BY task LIMIT ?1")?;
    let mut found = Vec::new();

    for task in statement.query_map([sql_number(MAX_TASKS)?], |row| row.get(0))? {
        found.push(task?);
    }

    Ok(found)
}

/// The last `count` entries, the newest first. Used by the tests, which
/// check that every question is in the journal (P3).
#[cfg(test)]
pub fn last(db: &UserDb, count: usize) -> Result<Vec<Entry>, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT * FROM ai_journal ORDER BY id DESC LIMIT ?1")?;
    let mut found = Vec::new();

    for row in statement.query_map([sql_number(count)?], |row| Ok(rows::entry(row)))? {
        found.push(row??);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of reading the journal of the AI.

    use super::*;

    use crate::db_old::ai::journal::create;
    use crate::test_support::db::journal_entry;
    use crate::test_support::standards::database;

    #[test]
    fn the_newest_entries_come_first_and_no_more_than_asked() {
        let test = database();
        create::entry(&test.db, &journal_entry("chat")).unwrap();
        create::entry(&test.db, &journal_entry("chapter")).unwrap();

        let entries = last(&test.db, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].task, "chapter");
        assert_eq!(entries[1].task, "chat");
        assert_eq!(last(&test.db, 1).unwrap().len(), 1);
    }

    #[test]
    fn the_list_is_filtered_by_task_and_words_and_paged() {
        let test = database();
        let mut found = journal_entry("assessment");
        found.answer = "The TSF meets FCS_CKM.1.".to_owned();
        create::entry(&test.db, &found).unwrap();
        create::entry(&test.db, &journal_entry("chat")).unwrap();

        let (all, total) = summaries(&test.db, None, "", 0).unwrap();
        assert_eq!((all.len(), total), (2, 2));
        assert_eq!(all[0].task, "chat", "the newest first");

        let (by_task, _) = summaries(&test.db, Some("assessment"), "", 0).unwrap();
        assert_eq!(by_task.len(), 1);
        let (by_words, total) = summaries(&test.db, None, "fcs_ckm", 0).unwrap();
        assert_eq!((by_words[0].task.as_str(), total), ("assessment", 1));
        assert!(summaries(&test.db, None, "", 1).unwrap().0.is_empty());

        assert_eq!(entry(&test.db, by_words[0].id).unwrap().unwrap().answer, "The TSF meets FCS_CKM.1.");
        assert_eq!(entry(&test.db, 99).unwrap(), None);
        assert_eq!(tasks(&test.db).unwrap(), ["assessment", "chat"]);
    }
}
