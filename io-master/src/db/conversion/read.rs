//! Reading the conversions.

use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::conversion::{Conversion, GroupPages, MAX_GROUPS, Stage};

use super::rows;

/// The conversion of catalog `uuid`; `None` when it has none (an imported
/// catalog, finished).
///
/// Called by: the commands of the wizard (`catalog_editor.rs`), and
/// [`finished`].
pub fn conversion(db: &UserDb, uuid: &str) -> Result<Option<Conversion>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM conversions WHERE catalog_uuid = ?1")?;

    statement.query_row([uuid], |row| Ok(rows::conversion(row))).optional()?.transpose()
}

/// Whether catalog `uuid` is finished: converted to its end, or never in
/// conversion (imported).
///
/// Called by: `views/evaluation_plan/evaluation_plan.rs`, which shows only
/// the finished catalogs.
pub fn finished(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    match conversion(db, uuid)? {
        Some(conversion) => Ok(conversion.stage == Stage::Finished),
        None => Ok(true),
    }
}

/// The groups of the conversion of catalog `uuid`, in the order of the
/// document; at most `MAX_GROUPS`.
///
/// Called by: the commands of the wizard (`catalog_editor.rs`).
pub fn groups(db: &UserDb, uuid: &str) -> Result<Vec<GroupPages>, DbError> {
    let connection = db.connection();
    let mut statement =
        connection.prepare("SELECT * FROM conversion_groups WHERE catalog_uuid = ?1 ORDER BY position LIMIT ?2")?;
    let limit = i64::try_from(MAX_GROUPS).map_err(|error| DbError::Inconsistent(error.to_string()))?;
    let mut found = Vec::new();

    for row in statement.query_map(rusqlite::params![uuid, limit], |row| Ok(rows::group(row)))? {
        found.push(row??);
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    //! Tests of reading the conversions.

    use super::*;

    use crate::db::conversion::{create, update};
    use crate::test_support::standards::database;

    #[test]
    fn a_catalog_without_a_conversion_is_finished() {
        let test = database();

        assert_eq!(conversion(&test.db, "imported").unwrap(), None);
        assert!(finished(&test.db, "imported").unwrap());

        create::conversion(&test.db, "made").unwrap();
        assert!(!finished(&test.db, "made").unwrap());

        update::stage(&test.db, "made", Stage::Finished).unwrap();
        assert!(finished(&test.db, "made").unwrap());
    }

    #[test]
    fn a_stage_the_program_never_writes_is_corrupt() {
        let test = database();

        create::conversion(&test.db, "c").unwrap();
        test.db.connection_for_tests().execute_batch("UPDATE conversions SET stage = 'later'").unwrap();

        assert!(matches!(conversion(&test.db, "c"), Err(DbError::Corrupt(_))));
    }
}
