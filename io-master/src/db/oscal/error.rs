//! The error of an operation on a stored document of OSCAL: the rules of
//! the stored documents (a UUID once, an id once in a document, the model
//! an operation needs) as the views and the work on catalogs tell them.

use std::fmt;

use crate::db::error::DbError;
use crate::model_old::oscal::NotOscal;
use crate::model_old::oscal::catalog::CatalogError;

/// Why an operation on a document of OSCAL was refused.
#[derive(Debug)]
pub enum OscalError {
    /// The file is not a document of OSCAL the program can take.
    NotOscal(NotOscal),
    /// A document with this UUID is stored already (its title).
    AlreadyStored(String),
    /// There is no document with this UUID.
    NotFound,
    /// The document is not of the model the operation needs (a catalog,
    /// ...).
    WrongModel,
    /// There is no group or control with this id in the document.
    NoSuchObject(String),
    /// Another group or control of the document has this id.
    IdTaken(String),
    /// What was given breaks a rule; the reason, for the evaluator.
    Invalid(String),
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
            OscalError::WrongModel => formatter.write_str("the OSCAL document is not of the model this needs"),
            OscalError::NoSuchObject(id) => write!(formatter, "the document has no group or control {id}"),
            OscalError::IdTaken(id) => write!(formatter, "another group or control of the document has the id {id}"),
            OscalError::Invalid(reason) => formatter.write_str(reason),
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

impl From<CatalogError> for OscalError {
    fn from(error: CatalogError) -> OscalError {
        match error {
            CatalogError::NoSuchObject(id) => OscalError::NoSuchObject(id),
            CatalogError::IdTaken(id) => OscalError::IdTaken(id),
            other => OscalError::Invalid(other.to_string()),
        }
    }
}
