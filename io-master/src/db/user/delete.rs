//! Removing secrets of an evaluator, and what earlier versions kept.

use rusqlite::Connection;

use crate::db::error::DbError;

use super::database::UserDb;

/// What earlier versions kept and this one no longer uses: the API
/// key of Anthropic and the chosen Claude model, replaced by the AI
/// connections (step 15 of docs/todo.md); the instructions of the
/// analysis kept as settings, replaced by `db/ai/instructions/` (step
/// 38; the user agreed that they go back to the default ones). Removed, the key
/// overwritten in the file, whenever a database is opened.
///
/// TODO (step 10 of docs/todo.md): remove before the release; no
/// database of a released version holds these.
const REMOVED: &str = "
    PRAGMA secure_delete = ON;
    DELETE FROM secrets WHERE name = 'anthropic.api_key';
    DELETE FROM settings WHERE name = 'ai.model';
    DELETE FROM settings WHERE name IN ('evaluation.analysis_instructions', 'evaluation.contents_instructions');
";

/// Removes [`REMOVED`]; called when a database is opened
/// (`database.rs`).
pub(super) fn remove_obsolete(connection: &Connection) -> Result<(), DbError> {
    connection.execute_batch(REMOVED)?;

    Ok(())
}

impl UserDb {
    /// Removes a setting; a setting that is not stored is not an
    /// error.
    pub fn remove_setting(&self, name: &str) -> Result<(), DbError> {
        self.connection().execute("DELETE FROM settings WHERE name = ?1", [name])?;

        Ok(())
    }

    /// Removes a secret; a secret that is not stored is not an
    /// error.
    ///
    /// SQLite marks the space of a removed row as free but does not
    /// overwrite it at once; `secure_delete` makes it overwrite the
    /// value with zeros, so that the old secret is not left in the
    /// (encrypted) file.
    pub fn remove_secret(&self, name: &str) -> Result<(), DbError> {
        self.connection().execute_batch("PRAGMA secure_delete = ON")?;
        self.connection().execute("DELETE FROM secrets WHERE name = ?1", [name])?;

        Ok(())
    }
}
