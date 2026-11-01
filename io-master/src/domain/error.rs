//! The error of `domain/`, the rules of the data: a change the rules do not
//! allow, a document that is no OSCAL catalog, or a failure of the database. The panels turn it into their own
//! error (their `error.rs`, `From<DomainError>`).

use std::fmt;

use crate::db::error::DbError;
use crate::domain::models::evaluation::oscal_standard_document::OscalError;

/// Why a rule of the data refused a change, or could not be checked.
#[derive(Debug)]
pub enum DomainError {
    /// The rule refuses it; holds the reason ("only a plan in planning is
    /// changed").
    Refused(String),
    /// The document of a standard is no catalog the program takes
    /// (`models/evaluation/oscal_standard_document.rs`).
    Oscal(OscalError),
    /// A database failed, or refused what was given to it.
    Db(DbError),
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::Refused(reason) => formatter.write_str(reason),
            DomainError::Oscal(error) => write!(formatter, "{error}"),
            DomainError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for DomainError {}

impl From<OscalError> for DomainError {
    fn from(error: OscalError) -> DomainError {
        DomainError::Oscal(error)
    }
}

impl From<DbError> for DomainError {
    fn from(error: DbError) -> DomainError {
        DomainError::Db(error)
    }
}
