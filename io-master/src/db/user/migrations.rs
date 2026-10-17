//! The changes of the layout of the database of an evaluator, by its
//! version (`PRAGMA user_version`, step 40 of docs/todo.md).
//!
//! A new database starts at version 1 (`encryption/sqlcipher.rs`,
//! `SCHEMA_VERSION`). When a database is opened, its missing tables
//! are made (`CREATE TABLE IF NOT EXISTS`, in `database.rs`) and then
//! every step below its version runs once, in one transaction, and
//! the version is raised. A database already at [`VERSION`] runs
//! nothing.
//!
//! ```text
//! 1 → 2   the column message_template of ai_connections (step 15);
//!         what earlier versions kept and this one no longer uses;
//!         names unique whatever their letter case
//! ```
//!
//! TODO (step 10 of docs/todo.md): before the release, the steps that
//! only serve the databases of the development go; a released version
//! starts its own history of versions.

use rusqlite::Connection;

use crate::db::error::DbError;

/// The version of the layout this program writes.
pub const VERSION: i64 = 2;

/// Brings the database to [`VERSION`].
pub(super) fn run(connection: &Connection) -> Result<(), DbError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version >= VERSION {
        return Ok(());
    }

    let transaction = connection.unchecked_transaction()?;

    if version < 2 {
        to_2(&transaction)?;
    }

    transaction.execute_batch(&format!("PRAGMA user_version = {VERSION}"))?;
    transaction.commit()?;

    Ok(())
}

/// Version 2.
fn to_2(connection: &Connection) -> Result<(), DbError> {
    // A table ai_connections made before the column existed (the same
    // day, during step 15).
    let has_template: bool = connection.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('ai_connections') WHERE name = 'message_template'",
        [],
        |row| row.get(0),
    )?;

    if !has_template {
        connection.execute_batch(
            "ALTER TABLE ai_connections ADD COLUMN message_template TEXT NOT NULL \
             DEFAULT '{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}'",
        )?;
    }

    // What earlier versions kept and this one no longer uses: the API
    // key of Anthropic and the chosen Claude model, replaced by the AI
    // connections (step 15); the instructions of the analysis kept as
    // settings, replaced by db/ai/instructions/ (step 38; the user
    // agreed that they go back to the default ones). The key is
    // overwritten in the file.
    connection.execute_batch(
        "PRAGMA secure_delete = ON;
         DELETE FROM secrets WHERE name = 'anthropic.api_key';
         DELETE FROM settings WHERE name = 'ai.model';
         DELETE FROM settings WHERE name IN ('evaluation.analysis_instructions', 'evaluation.contents_instructions');",
    )?;

    // Names unique whatever their letter case, as the program checks
    // them: kept by the database itself. (An index, because the
    // collation of an existing column cannot be changed.)
    connection.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS projects_name_nocase ON projects (name COLLATE NOCASE);
         CREATE UNIQUE INDEX IF NOT EXISTS evaluations_name_nocase ON evaluations (name COLLATE NOCASE);
         CREATE UNIQUE INDEX IF NOT EXISTS ai_connections_name_nocase ON ai_connections (name COLLATE NOCASE);",
    )?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "migrations_tests.rs"]
mod tests;
