//! A file of OSCAL (JSON) read into the database, and written from it:
//! the document checked against the schema of NIST both ways, a UUID
//! stored once.

use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::user::database::UserDb;
use crate::model::oscal::{NotOscal, OscalDocument, OscalSummary};

/// The largest file of OSCAL that is read, in bytes (the catalog of NIST
/// SP 800-53 has about 10 MiB).
pub const MAX_FILE: u64 = 64 * 1024 * 1024;

/// Stores the document in the text of a file of OSCAL; returns what
/// identifies it. Refused when it is not OSCAL, breaks the schema of
/// NIST, or is stored already.
pub fn import(db: &UserDb, text: &str) -> Result<OscalSummary, OscalError> {
    let document = match OscalDocument::from_json(text) {
        Ok(document) => document,
        Err(error) => return Err(OscalError::NotOscal(error)),
    };

    if let Some(stored) = db_oscal::read::summary(db, document.uuid())? {
        return Err(OscalError::AlreadyStored(stored.title));
    }

    db_oscal::create::document(db, &document)?;

    Ok(document.summary())
}

/// The text of the file of document `uuid`: its JSON as OSCAL defines
/// it, checked against the schema of NIST, indented for a person.
pub fn export(db: &UserDb, uuid: &str) -> Result<String, OscalError> {
    let Some(document) = db_oscal::read::document(db, uuid)? else {
        return Err(OscalError::NotFound);
    };

    let value = match document.to_json() {
        Ok(value) => value,
        Err(error) => return Err(OscalError::NotOscal(error)),
    };

    match serde_json::to_string_pretty(&value) {
        Ok(text) => Ok(text),
        Err(error) => Err(OscalError::NotOscal(NotOscal::Types(error.to_string()))),
    }
}

/// Deletes document `uuid` with its evaluation support; returns whether
/// it existed.
pub fn delete(db: &UserDb, uuid: &str) -> Result<bool, DbError> {
    db_evaluation_support::delete::of_document(db, uuid)?;
    db_oscal::delete::document(db, uuid)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
