//! Reading the evaluation support.

use rusqlite::{OptionalExtension, named_params};

use crate::db_old::error::DbError;
use crate::db_old::user::database::UserDb;
use crate::model_old::evaluation_support::{Proposal, Quote};

use super::rows::{proposal_from_row, quote_from_row};

/// The quotes of document `document_uuid` (all of them, or of one
/// object), in the order they were stored.
pub fn quotes(db: &UserDb, document_uuid: &str, object_id: Option<&str>) -> Result<Vec<Quote>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT * FROM support_quotes WHERE document_uuid = :document AND (:object IS NULL OR object_id = :object) \
         ORDER BY id",
    )?;

    Ok(statement
        .query_map(named_params! { ":document": document_uuid, ":object": object_id }, quote_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// Whether a quote names document `source_document_id` of a standard.
pub fn quotes_source(db: &UserDb, source_document_id: i64) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT EXISTS (SELECT 1 FROM support_quotes WHERE source_document_id = :source)",
        named_params! { ":source": source_document_id },
        |row| row.get(0),
    )?)
}

/// The proposals waiting for document `document_uuid`, the oldest first.
pub fn proposals(db: &UserDb, document_uuid: &str) -> Result<Vec<Proposal>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT * FROM support_proposals WHERE document_uuid = :document ORDER BY id")?;

    Ok(statement
        .query_map(named_params! { ":document": document_uuid }, proposal_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// Proposal `id`; `None` when there is no such.
pub fn proposal(db: &UserDb, id: i64) -> Result<Option<Proposal>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT * FROM support_proposals WHERE id = :id", named_params! { ":id": id }, proposal_from_row)
        .optional()?)
}

#[cfg(test)]
mod tests {
    //! Tests of reading the evaluation support.

    use super::*;

    use crate::db_old::evaluation_support::create;
    use crate::test_support::db::{quote_draft, stored_proposal};
    use crate::test_support::standards::{database, load};

    #[test]
    fn the_quotes_of_a_document_or_of_one_of_its_objects_are_read() {
        let test = database();
        let source = load(&test.db, b"%PDF support");
        let draft = quote_draft(source, "The developer shall");
        let id = create::quote(&test.db, "doc", "ADV_FSP.4", &draft).unwrap();
        create::quote(&test.db, "doc", "ATE_IND.2", &draft).unwrap();

        assert_eq!(quotes(&test.db, "doc", None).unwrap().len(), 2);
        assert_eq!(quotes(&test.db, "doc", Some("ADV_FSP.4")).unwrap()[0].id, id);
    }

    #[test]
    fn a_document_quoted_is_told_from_one_that_is_not() {
        let test = database();
        let quoted = load(&test.db, b"%PDF support");
        let other = load(&test.db, b"%PDF other");
        create::quote(&test.db, "doc", "ADV_FSP.4", &quote_draft(quoted, "The developer shall")).unwrap();

        assert!(quotes_source(&test.db, quoted).unwrap());
        assert!(!quotes_source(&test.db, other).unwrap());
    }

    #[test]
    fn the_proposals_of_a_document_are_read() {
        let test = database();
        let source = load(&test.db, b"%PDF proposals");
        let id = stored_proposal(&test.db, source);

        assert_eq!(proposals(&test.db, "doc").unwrap().len(), 1);
        assert!(proposal(&test.db, id).unwrap().is_some());
        assert!(proposal(&test.db, id + 1).unwrap().is_none());
    }
}
