//! What both databases do with a connection just opened, before it is
//! used: the same wait for a busy database, and their tables made when
//! they are missing. Each database gives only the list of its tables
//! (`models/schema.rs`).
//!
//! ```text
//! user_database.rs   ──sqlcipher::open(path, key)──► Connection ──prepare(USER_TABLES)───► UserDatabase
//! system_database.rs ──sqlcipher::open_plain(path)─► Connection ──prepare(SYSTEM_TABLES)─► SystemDatabase
//! ```

use std::time::Duration;

use rusqlite::Connection;

use crate::db::error::DbError;

/// How long an operation waits for another that holds the database, before
/// it fails: two commands of the window may use the database at once, and
/// the second waits for the first instead of failing at once.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Prepares a connection just opened: the wait for a busy database, and
/// every table of `tables` that is missing.
///
/// Input:
/// - `connection`: the connection to a database file, just opened (and, for
///   the database of the user, unlocked with its key).
/// - `tables`: the `SCHEMA` of every table of that database
///   (`schema.rs`: `USER_TABLES` or `SYSTEM_TABLES`); each is a
///   `CREATE TABLE IF NOT EXISTS`, so a table that exists stays as it is.
///
/// Output: nothing; the database holds every table of `tables`. An error of
/// SQLite when a statement fails.
///
/// Called by: `user_database.rs`: `UserDatabase::from`;
/// `system_database.rs`: `SystemDatabase::from`.
pub(in crate::db::storage) fn prepare(connection: &Connection, tables: &[&str]) -> Result<(), DbError> {
    ensure!(!tables.is_empty(), DbError::Inconsistent("a database prepared without its tables".to_owned()));

    connection.busy_timeout(BUSY_TIMEOUT)?;

    // The list is bounded: it is a constant of schema.rs.
    for table in tables {
        connection.execute_batch(table)?;
    }

    Ok(())
}
