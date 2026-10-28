//! Deleting from the evaluation support.

use rusqlite::named_params;

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;

/// Deletes quote `id`; returns whether it existed.
pub fn quote(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM support_quotes WHERE id = :id", named_params! { ":id": id })? > 0)
}

/// Deletes the quotes of the objects `object_ids` of document
/// `document_uuid` (the objects were removed from it).
pub fn quotes_of(db: &UserDb, document_uuid: &str, object_ids: &[String]) -> Result<(), DbError> {
    for object in object_ids {
        db.connection().execute(
            "DELETE FROM support_quotes WHERE document_uuid = :document AND object_id = :object",
            named_params! { ":document": document_uuid, ":object": object },
        )?;
    }

    Ok(())
}

/// Deletes proposal `id` (accepted into the document, or rejected);
/// returns whether it existed.
pub fn proposal(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM support_proposals WHERE id = :id", named_params! { ":id": id })? > 0)
}

/// Deletes everything of the evaluation support of document
/// `document_uuid` (the document was deleted).
pub fn of_document(db: &UserDb, document_uuid: &str) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM support_quotes WHERE document_uuid = :document",
        named_params! { ":document": document_uuid },
    )?;
    transaction.execute(
        "DELETE FROM support_proposals WHERE document_uuid = :document",
        named_params! { ":document": document_uuid },
    )?;
    transaction.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of deleting from the evaluation support.

    use super::*;

    use crate::db_old::evaluation_support::{create, read};
    use crate::test_support::db::{quote_draft, stored_proposal};
    use crate::test_support::standards::{database, load};

    #[test]
    fn the_quotes_of_objects_and_one_quote_are_deleted() {
        let test = database();
        let source = load(&test.db, b"%PDF support");
        let draft = quote_draft(source, "The developer shall");
        let id = create::quote(&test.db, "doc", "ADV_FSP.4", &draft).unwrap();
        create::quote(&test.db, "doc", "ATE_IND.2", &draft).unwrap();

        quotes_of(&test.db, "doc", &["ATE_IND.2".to_owned()]).unwrap();
        assert_eq!(read::quotes(&test.db, "doc", None).unwrap().len(), 1);

        assert!(quote(&test.db, id).unwrap());
        assert!(read::quotes(&test.db, "doc", None).unwrap().is_empty());
    }

    #[test]
    fn the_proposals_of_a_document_are_deleted_with_it() {
        let test = database();
        let source = load(&test.db, b"%PDF proposals");
        stored_proposal(&test.db, source);

        of_document(&test.db, "doc").unwrap();
        assert!(read::proposals(&test.db, "doc").unwrap().is_empty());
    }

    #[test]
    fn one_proposal_is_deleted_once() {
        let test = database();
        let source = load(&test.db, b"%PDF proposals");
        let id = stored_proposal(&test.db, source);

        assert!(proposal(&test.db, id).unwrap());
        assert!(read::proposals(&test.db, "doc").unwrap().is_empty());
        assert!(!proposal(&test.db, id).unwrap());
    }
}
