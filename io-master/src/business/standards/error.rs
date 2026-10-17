//! The error of an operation on a document of a standard.

use std::fmt;

use crate::db::error::DbError;
use crate::model::text::Invalid;

/// Why a document was not stored or removed.
#[derive(Debug)]
pub enum DocumentError {
    /// A field breaks a rule; holds the reason.
    Invalid(String),
    /// The same file is loaded already, as this document.
    Duplicate(i64),
    /// A test project or an evaluation project uses the document.
    InUse,
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentError::Invalid(reason) => formatter.write_str(reason),
            DocumentError::Duplicate(_) => formatter.write_str("this file is loaded already"),
            DocumentError::InUse => formatter.write_str(
                "a test project or an evaluation uses this document; remove it from the test project, \
                     or delete the evaluation, first",
            ),
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
