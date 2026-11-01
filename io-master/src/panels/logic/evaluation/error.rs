//! The error of the panel of the evaluation: what it shows could not be
//! read from the database. The rules of an evaluation refuse in
//! `objects/evaluation/` (`DomainError`), its assessment there too
//! (`AiError`).

use std::fmt;

use crate::infrastructure::storage::error::DbError;

/// Why what the panel of the evaluation shows could not be read.
#[derive(Debug)]
pub enum EvaluationError {
    /// A database failed, or holds what the program never writes.
    Db(DbError),
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EvaluationError {}

impl From<DbError> for EvaluationError {
    fn from(error: DbError) -> EvaluationError {
        EvaluationError::Db(error)
    }
}
