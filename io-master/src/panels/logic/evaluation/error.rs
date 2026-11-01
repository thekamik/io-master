//! The error of the panel of the evaluation: a change refused, a quote the
//! way to the AI could not check, a rule of the data, a failure of the
//! database. The commands of
//! the panel tell its kinds apart for the page.

use std::fmt;

use crate::ai::error::AiError;
use crate::db::error::DbError;
use crate::domain::error::DomainError;

/// Why a change of an evaluation was refused or failed.
#[derive(Debug)]
pub enum EvaluationError {
    /// The change does not hold together; holds the reason (a quote not
    /// found in its document, a verdict of no plan).
    Refused(String),
    /// The check of a quote in its documents failed (`ai/`).
    Ai(AiError),
    /// A database failed, or refused what was given to it.
    Db(DbError),
    /// A rule of the data refused the change, or could not be checked
    /// (`domain/`).
    Domain(DomainError),
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationError::Refused(reason) => formatter.write_str(reason),
            EvaluationError::Ai(error) => write!(formatter, "{error}"),
            EvaluationError::Db(error) => write!(formatter, "{error}"),
            EvaluationError::Domain(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EvaluationError {}

impl From<AiError> for EvaluationError {
    fn from(error: AiError) -> EvaluationError {
        EvaluationError::Ai(error)
    }
}

impl From<DbError> for EvaluationError {
    fn from(error: DbError) -> EvaluationError {
        EvaluationError::Db(error)
    }
}

impl From<DomainError> for EvaluationError {
    fn from(error: DomainError) -> EvaluationError {
        EvaluationError::Domain(error)
    }
}
