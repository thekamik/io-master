//! Starting the conversion of a catalog.

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model_old::conversion::Stage;

/// Starts the conversion of catalog `uuid`, at the stage of the table of
/// contents.
///
/// Called by: `views/converter/new_catalog/made.rs`, when a catalog is made
/// from the documents of a standard.
pub fn conversion(db: &UserDb, uuid: &str) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO conversions (catalog_uuid, stage) VALUES (:uuid, :stage)",
        named_params! { ":uuid": uuid, ":stage": Stage::Contents.name() },
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of starting a conversion.

    use super::*;

    use crate::db::conversion::read;
    use crate::model_old::conversion::Conversion;
    use crate::test_support::standards::database;

    #[test]
    fn a_conversion_starts_at_the_table_of_contents_once() {
        let test = database();

        conversion(&test.db, "c-1").unwrap();

        assert_eq!(
            read::conversion(&test.db, "c-1").unwrap(),
            Some(Conversion { stage: Stage::Contents, contents: None })
        );
        assert!(conversion(&test.db, "c-1").is_err());
    }
}
