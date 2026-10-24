//! Reading the journal of the AI.

#[cfg(test)]
use crate::db::error::DbError;
#[cfg(test)]
use crate::db::user::database::UserDb;

#[cfg(test)]
use super::rows::{COLUMNS, Entry, entry_from_row};

/// The last `count` entries, the newest first. Used by the tests only (they
/// check that every question is in the journal, P3); the program writes the
/// journal and has no page that reads it yet (a reminder of `docs/todo.md`).
#[cfg(test)]
pub fn last(db: &UserDb, count: usize) -> Result<Vec<Entry>, DbError> {
    let mut statement =
        db.connection().prepare(&format!("SELECT {COLUMNS} FROM ai_journal ORDER BY id DESC LIMIT ?1"))?;

    Ok(statement.query_map([count as i64], entry_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    //! Tests of reading the journal of the AI.

    use super::*;

    use crate::db::ai::journal::create;
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
}
