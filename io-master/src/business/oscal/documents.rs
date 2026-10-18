//! The documents of OSCAL as a resource (step 41.2): loaded from files
//! (of NIST, of a laboratory, of the program), listed, written to files,
//! deleted. Every document is checked against the schema of NIST when
//! it is read and again when it is stored or written
//! (`model/oscal/schema.rs`).
//!
//! ```text
//! list()     GET     /oscal
//! get()      GET     /oscal/{uuid}
//! import()   POST    /oscal            a file of OSCAL, as JSON
//! export()   GET     /oscal/{uuid}/file
//! delete()   DELETE  /oscal/{uuid}
//! ```

use std::fmt;

use crate::db::error::DbError;
use crate::db::oscal as db_oscal;
use crate::db::user::database::UserDb;
use crate::model::oscal::{NotOscal, OscalDocument, OscalSummary};

/// Why an operation on a document of OSCAL was refused.
#[derive(Debug)]
pub enum OscalError {
    /// The file is not a document of OSCAL the program can take.
    NotOscal(NotOscal),
    /// A document with this UUID is stored already (its title).
    AlreadyStored(String),
    /// There is no document with this UUID.
    NotFound,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for OscalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OscalError::NotOscal(error) => write!(formatter, "{error}"),
            OscalError::AlreadyStored(title) => {
                write!(formatter, "this document is loaded already, as \"{title}\" (the same UUID)")
            }
            OscalError::NotFound => formatter.write_str("the OSCAL document does not exist"),
            OscalError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for OscalError {}

impl From<DbError> for OscalError {
    fn from(error: DbError) -> OscalError {
        match error {
            DbError::NotOscal(error) => OscalError::NotOscal(error),
            other => OscalError::Db(other),
        }
    }
}

/// Every document, by model and title.
pub fn list(db: &UserDb) -> Result<Vec<OscalSummary>, DbError> {
    db_oscal::read::summaries(db)
}

/// Document `uuid`; `None` when there is no such.
pub fn get(db: &UserDb, uuid: &str) -> Result<Option<OscalDocument>, DbError> {
    db_oscal::read::document(db, uuid)
}

/// Loads a document from the text of a file of OSCAL (JSON); returns
/// what identifies it. Refused when it is not OSCAL, breaks the schema
/// of NIST, or is stored already.
pub fn import(db: &UserDb, text: &str) -> Result<OscalSummary, OscalError> {
    let document = OscalDocument::from_json(text).map_err(OscalError::NotOscal)?;

    if let Some(stored) = db_oscal::read::summary(db, document.uuid())? {
        return Err(OscalError::AlreadyStored(stored.title));
    }

    db_oscal::create::document(db, &document)?;

    Ok(document.summary())
}

/// The file of document `uuid`: its JSON as OSCAL defines it, checked
/// against the schema of NIST, indented for a person to read.
pub fn export(db: &UserDb, uuid: &str) -> Result<String, OscalError> {
    let document = get(db, uuid)?.ok_or(OscalError::NotFound)?;
    let value = document.to_json().map_err(OscalError::NotOscal)?;

    serde_json::to_string_pretty(&value).map_err(|error| OscalError::NotOscal(NotOscal::Types(error.to_string())))
}

/// Deletes document `uuid`; returns whether it existed.
pub fn delete(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    db_oscal::delete::document(db, uuid)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "documents_tests.rs"]
mod tests;
