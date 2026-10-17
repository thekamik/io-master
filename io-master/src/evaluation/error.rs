//! The error of an operation on an evaluation project.

use std::fmt;

use crate::db::error::DbError;
use crate::model::test_projects::rules::SchemeError;
use crate::model::text::Invalid;

/// Why an operation on an evaluation was refused.
#[derive(Debug)]
pub enum EvaluationError {
    /// A field is wrong: the field and the reason.
    Invalid(&'static str, String),
    /// Another evaluation has this name.
    NameTaken,
    /// The evaluation, test project, tile or proposal does not exist.
    NotFound(&'static str),
    /// Results belong to requirements only.
    NotARequirement,
    /// The test project breaks a rule of its own (a broken file).
    Scheme(SchemeError),
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationError::Invalid(_, reason) => formatter.write_str(reason),
            EvaluationError::NameTaken => formatter.write_str("another evaluation has this name"),
            EvaluationError::NotFound(what) => write!(formatter, "the {what} does not exist"),
            EvaluationError::NotARequirement => formatter.write_str("results belong to requirements only"),
            EvaluationError::Scheme(error) => write!(formatter, "{error}"),
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

impl From<SchemeError> for EvaluationError {
    fn from(error: SchemeError) -> EvaluationError {
        EvaluationError::Scheme(error)
    }
}

impl From<Invalid> for EvaluationError {
    fn from(error: Invalid) -> EvaluationError {
        EvaluationError::Invalid(error.field, error.reason)
    }
}
