//! Adding an entry to the journal of the AI (the only way an entry
//! comes into being; none is ever changed or removed).

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::rows::Entry;

/// Adds `entry` (its `id` and `time` are ignored: the database gives
/// them); returns its number.
pub fn entry(db: &UserDb, entry: &Entry) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO ai_journal (task, role, connection, model, request, answer, ending, note, input_tokens, \
         output_tokens) VALUES (:task, :role, :connection, :model, :request, :answer, :ending, :note, :input_tokens, :\
         output_tokens)",
        named_params! { ":task": entry.task, ":role": entry.role, ":connection": entry.connection, ":model": entry.model, ":request": entry.request, ":answer": entry.answer, ":ending": entry.ending, ":note": entry.note, ":input_tokens": entry.input_tokens as i64, ":\
         output_tokens": entry.output_tokens as i64 },
    )?;

    Ok(db.connection().last_insert_rowid())
}

#[cfg(test)]
mod tests {
    //! Tests of adding an entry to the journal of the AI.

    use super::*;

    use crate::db::ai::journal::read;
    use crate::test_support::db::journal_entry;
    use crate::test_support::standards::database;

    #[test]
    fn an_entry_is_stored_as_it_was_given_with_its_time() {
        let test = database();

        entry(&test.db, &journal_entry("chat")).unwrap();

        let stored = &read::last(&test.db, 10).unwrap()[0];
        assert_eq!(*stored, Entry { id: stored.id, time: stored.time, ..journal_entry("chat") });
        assert!(stored.time > 1_700_000_000);
    }
}
