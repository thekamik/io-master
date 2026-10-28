//! Deleting a document of OSCAL. Its text is overwritten in the file,
//! not only marked free: a plan or results may hold what a manufacturer
//! gave under a non-disclosure agreement.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;

/// Deletes document `uuid`; returns whether it existed.
pub fn document(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute_batch("PRAGMA secure_delete = ON")?;
    let deleted =
        transaction.execute("DELETE FROM oscal_documents WHERE uuid = :uuid", named_params! { ":uuid": uuid })? > 0;
    transaction.commit()?;

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    //! Tests of deleting a document of OSCAL.

    use super::*;

    use crate::db_old::oscal::{create, read};
    use crate::test_support::db::basic_catalog;
    use crate::test_support::standards::database;

    #[test]
    fn a_document_is_deleted_once() {
        let test = database();
        let stored = basic_catalog();
        let uuid = stored.uuid().to_owned();
        create::document(&test.db, &stored).unwrap();

        assert!(document(&test.db, &uuid).unwrap());
        assert!(read::document(&test.db, &uuid).unwrap().is_none());
        assert!(!document(&test.db, &uuid).unwrap());
    }
}
