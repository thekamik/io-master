//! What is stored of a document of a standard, with its rules: a file is
//! loaded once (its SHA-256), the fields are checked in their normal form
//! (`model/standards.rs`), and a document an OSCAL document quotes stays.
//! The SQL is in `db/standards/`; the licensed text is overwritten in the
//! file when a document is deleted (`db/standards/delete.rs`).

use std::fmt;

use crate::app::error::UiError;
use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::standards as db_standards;
use crate::db::user::database::UserDb;
use crate::model::standards::{DocumentFields, sha256_hex};
use crate::model::text::Invalid;

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

impl From<DocumentError> for UiError {
    fn from(error: DocumentError) -> UiError {
        match error {
            DocumentError::Db(error) => UiError::Failed(error.to_string()),
            other => UiError::Invalid(other.to_string()),
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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "stored_tests.rs"]
mod tests;
