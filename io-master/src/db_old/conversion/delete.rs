//! Deleting a conversion.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;

/// Deletes the conversion of catalog `uuid` and its groups, in one
/// transaction (the catalog was deleted).
///
/// Called by: `views/evaluation_plan/oscal_documents/file.rs` (`delete`).
pub fn of_catalog(db: &UserDb, uuid: &str) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM conversion_groups WHERE catalog_uuid = :uuid", named_params! { ":uuid": uuid })?;
    transaction.execute("DELETE FROM conversions WHERE catalog_uuid = :uuid", named_params! { ":uuid": uuid })?;
    transaction.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of deleting a conversion.

    use super::*;

    use crate::db_old::conversion::{create, read, update};
    use crate::model_old::conversion::{GroupPages, GroupState};
    use crate::test_support::standards::database;

    #[test]
    fn the_conversion_goes_with_its_groups() {
        let test = database();
        create::conversion(&test.db, "c").unwrap();
        let group = GroupPages { group_id: "g".to_owned(), pages: None, state: GroupState::Todo };
        update::groups(&test.db, "c", &[group]).unwrap();

        of_catalog(&test.db, "c").unwrap();

        assert_eq!(read::conversion(&test.db, "c").unwrap(), None);
        assert!(read::groups(&test.db, "c").unwrap().is_empty());
    }
}
