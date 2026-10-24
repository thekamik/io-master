//! The rows of the conversions and the types of `model/conversion.rs` they
//! become. A value the program never writes is `DbError::Corrupt`.

use crate::db::error::{Corrupt, DbError};
use crate::model::conversion::{Conversion, GroupPages, GroupState, Pages, Stage};

/// Pages from three columns that are all set or all NULL.
///
/// Called by: [`conversion`], [`group`].
fn pages(
    table: &'static str,
    document: Option<i64>,
    first: Option<u32>,
    last: Option<u32>,
) -> Result<Option<Pages>, DbError> {
    match (document, first, last) {
        (Some(document_id), Some(first), Some(last)) => Ok(Some(Pages { document_id, first, last })),
        (None, None, None) => Ok(None),
        _ => Err(DbError::Corrupt(Corrupt { table, column: "pages", value: "partly NULL".to_owned() })),
    }
}

/// A row of `conversions`.
///
/// Called by: `read::conversion`.
pub(super) fn conversion(row: &rusqlite::Row<'_>) -> Result<Conversion, DbError> {
    let stage: String = row.get("stage")?;
    let Some(stage) = Stage::from_name(&stage) else {
        return Err(DbError::Corrupt(Corrupt { table: "conversions", column: "stage", value: stage }));
    };

    let contents =
        pages("conversions", row.get("contents_document")?, row.get("contents_first")?, row.get("contents_last")?)?;

    Ok(Conversion { stage, contents })
}

/// A row of `conversion_groups`.
///
/// Called by: `read::groups`.
pub(super) fn group(row: &rusqlite::Row<'_>) -> Result<GroupPages, DbError> {
    let state: String = row.get("state")?;
    let Some(state) = GroupState::from_name(&state) else {
        return Err(DbError::Corrupt(Corrupt { table: "conversion_groups", column: "state", value: state }));
    };

    let pages = pages("conversion_groups", row.get("document_id")?, row.get("first_page")?, row.get("last_page")?)?;

    Ok(GroupPages { group_id: row.get("group_id")?, pages, state })
}
