//! What is stored of a document of a standard, with its rules: a file is
//! loaded once (its SHA-256), the fields are checked in their normal form
//! (`model/standards.rs`), and a document an OSCAL document quotes stays.
//! The SQL is in `db/standards/`; the licensed text is overwritten in the
//! file when a document is deleted (`db/standards/delete.rs`).

use std::fmt;

use crate::app::error::AppError;
use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model_old::standards::{DocumentFields, sha256_hex};
use crate::model_old::text::Invalid;

/// Why a document was not stored, changed or removed.
#[derive(Debug)]
pub enum DocumentError {
    /// A field breaks a rule; holds the reason.
    Invalid(String),
    /// The same file is loaded already, as this document.
    Duplicate(i64),
    /// An OSCAL document quotes the document (the evaluation support).
    InUse,
    /// There is no such document.
    NotFound,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentError::Invalid(reason) => formatter.write_str(reason),
            DocumentError::Duplicate(_) => formatter.write_str("this file is loaded already"),
            DocumentError::InUse => formatter
                .write_str("an OSCAL document names this document as its source; remove it from that document first"),
            DocumentError::NotFound => formatter.write_str("the document does not exist"),
            DocumentError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for DocumentError {}

impl From<DbError> for DocumentError {
    fn from(error: DbError) -> DocumentError {
        DocumentError::Db(error)
    }
}

impl From<Invalid> for DocumentError {
    fn from(error: Invalid) -> DocumentError {
        DocumentError::Invalid(error.reason)
    }
}

impl From<DocumentError> for AppError {
    fn from(error: DocumentError) -> AppError {
        match error {
            DocumentError::Db(error) => AppError::Failed(error.to_string()),
            other => AppError::Invalid(other.to_string()),
        }
    }
}

/// Stores a document: the file and the text of its pages (one text per
/// page, in order). Refused when the same file is loaded already.
pub fn create(
    db: &UserDb,
    fields: &DocumentFields,
    file_name: &str,
    pdf: &[u8],
    pages: &[String],
) -> Result<i64, DocumentError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    fields.check()?;

    ensure!(!pages.is_empty(), DocumentError::Invalid("the document has no pages".to_owned()));

    let sha256 = sha256_hex(pdf);

    if let Some(existing) = db_standards::read::by_sha256(db, &sha256)? {
        return Err(DocumentError::Duplicate(existing.id));
    }

    Ok(db_standards::create::document(db, fields, file_name, &sha256, pdf, pages)?)
}

/// Changes what the evaluator told about document `id`.
pub fn update(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<(), DocumentError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    fields.check()?;

    ensure!(db_standards::update::document(db, id, fields)?, DocumentError::NotFound);

    Ok(())
}

/// Removes document `id`; refused while a quote of an OSCAL document
/// names it.
pub fn delete(db: &UserDb, id: i64) -> Result<(), DocumentError> {
    ensure!(!db_evaluation_support::read::quotes_source(db, id)?, DocumentError::InUse);
    ensure!(db_standards::delete::document(db, id)?, DocumentError::NotFound);

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `stored.rs`.

    use super::*;

    use std::fs;

    use crate::test_support::standards::{PAGES, database, fields, load};

    #[test]
    fn a_document_is_stored_with_its_pages_and_file() {
        let test = database();
        let id = load(&test.db, b"%PDF-1.7 one");

        let document = db_standards::read::document(&test.db, id).unwrap().unwrap();
        assert_eq!(document.fields, fields());
        assert_eq!(document.page_count, 3);
        assert_eq!(document.sha256, sha256_hex(b"%PDF-1.7 one"));
        assert_eq!(db_standards::read::page(&test.db, id, 2).unwrap().as_deref(), Some(PAGES[1]));
        assert_eq!(db_standards::read::page(&test.db, id, 4).unwrap(), None);
    }

    #[test]
    fn the_same_file_is_loaded_once() {
        let test = database();
        let id = load(&test.db, b"%PDF same");

        let again = create(&test.db, &fields(), "copy.pdf", b"%PDF same", &["x".to_owned()]);

        assert!(matches!(again, Err(DocumentError::Duplicate(existing)) if existing == id));
    }

    #[test]
    fn a_document_needs_a_title_and_pages() {
        let test = database();
        let untitled = DocumentFields { title: " ".to_owned(), ..fields() };

        assert!(matches!(
            create(&test.db, &untitled, "a.pdf", b"a", &["x".to_owned()]),
            Err(DocumentError::Invalid(_))
        ));
        assert!(matches!(create(&test.db, &fields(), "a.pdf", b"a", &[]), Err(DocumentError::Invalid(_))));
    }

    #[test]
    fn the_fields_are_changed_and_the_document_deleted() {
        let test = database();
        let id = load(&test.db, b"%PDF change");

        let changed = DocumentFields { version: "CC:2022 R2".to_owned(), ..fields() };
        update(&test.db, id, &changed).unwrap();
        assert_eq!(db_standards::read::document(&test.db, id).unwrap().unwrap().fields, changed);

        delete(&test.db, id).unwrap();
        assert!(db_standards::read::document(&test.db, id).unwrap().is_none());
        assert!(matches!(delete(&test.db, id), Err(DocumentError::NotFound)));
        assert!(matches!(update(&test.db, id, &changed), Err(DocumentError::NotFound)));
    }

    #[test]
    fn the_text_cannot_be_read_from_the_file() {
        let test = database();
        load(&test.db, b"%PDF secret");
        let path = crate::db::user::database::path(test.directory.path(), 1);

        let bytes = fs::read(path).unwrap();
        assert!(!bytes.windows(17).any(|window| window == b"ADV_FSP.4 Complet"));
    }
}
