//! Reading the documents of OSCAL.

use rusqlite::{OptionalExtension, named_params};

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::oscal::catalog::Catalog;
use crate::model_old::oscal::{OscalDocument, OscalSummary};

use super::error::OscalError;

use super::rows::{document_from_row, summary_from_row};

/// Every document, by model and title.
pub fn summaries(db: &UserDb) -> Result<Vec<OscalSummary>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT uuid, model, title, version, oscal_version, last_modified FROM oscal_documents \
         ORDER BY model, title COLLATE NOCASE, uuid",
    )?;

    Ok(statement.query_map([], summary_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// What identifies document `uuid`; `None` when there is no such.
pub fn summary(db: &UserDb, uuid: &str) -> Result<Option<OscalSummary>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT uuid, model, title, version, oscal_version, last_modified FROM oscal_documents WHERE uuid = :uuid",
            named_params! { ":uuid": uuid },
            summary_from_row,
        )
        .optional()?)
}

/// Document `uuid`; `None` when there is no such.
pub fn document(db: &UserDb, uuid: &str) -> Result<Option<OscalDocument>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT json FROM oscal_documents WHERE uuid = :uuid",
            named_params! { ":uuid": uuid },
            document_from_row,
        )
        .optional()?)
}

/// Catalog `uuid`; an error when there is no such document, or it is not
/// a catalog.
pub fn catalog(db: &UserDb, uuid: &str) -> Result<Catalog, OscalError> {
    match document(db, uuid)? {
        Some(OscalDocument::Catalog(catalog)) => Ok(catalog),
        Some(_) => Err(OscalError::WrongModel),
        None => Err(OscalError::NotFound),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of reading the documents of OSCAL.

    use super::*;

    use crate::db_old::oscal::create;
    use crate::model_old::oscal::OscalModel;
    use crate::test_support::db::basic_catalog;
    use crate::test_support::standards::database;

    #[test]
    fn a_stored_document_is_read_with_its_summary() {
        let test = database();
        let stored = basic_catalog();
        create::document(&test.db, &stored).unwrap();

        let all = summaries(&test.db).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].model, OscalModel::Catalog);
        assert!(summary(&test.db, stored.uuid()).unwrap().is_some());
        assert_eq!(document(&test.db, stored.uuid()).unwrap().unwrap(), stored);
    }

    #[test]
    fn a_document_that_was_never_stored_is_none() {
        let test = database();

        assert!(document(&test.db, "00000000-0000-4000-8000-000000000000").unwrap().is_none());
    }
}
