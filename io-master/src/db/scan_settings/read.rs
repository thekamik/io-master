//! Reading the settings of the scanner.

use rusqlite::{OptionalExtension, Row, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

use super::{COLUMNS, PORT_TYPES};

/// The two stored texts; `None` for one never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScanSettings {
    pub port_types: Option<String>,
    pub columns: Option<String>,
}

/// Both texts, read by one statement, so that they always belong to
/// the same update.
pub fn texts(db: &UserDb) -> Result<StoredScanSettings, DbError> {
    let stored = db
        .connection()
        .query_row(
            "SELECT (SELECT value FROM settings WHERE name = :port_types) AS port_types,
                    (SELECT value FROM settings WHERE name = :columns) AS columns",
            named_params! { ":port_types": PORT_TYPES, ":columns": COLUMNS },
            stored_from_row,
        )
        .optional()?;

    // The statement always gives one row; none is a fault of SQLite.
    stored.ok_or(DbError::Sql("the settings of the scanner gave no row".to_owned()))
}

/// The row of [`texts`], its columns read by name.
fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredScanSettings> {
    Ok(StoredScanSettings { port_types: row.get("port_types")?, columns: row.get("columns")? })
}
