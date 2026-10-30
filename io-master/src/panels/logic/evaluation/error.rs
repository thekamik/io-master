//! The error of the panel of the evaluation: a change refused, a file of a
//! document that cannot be read, a question the way to the AI refused, a
//! failure of the database. The commands of
//! the panel tell its kinds apart for the page.

use std::fmt;

use crate::ai::error::AiError;
use crate::db::error::DbError;
use crate::pdf::poppler::PdfError;

/// Why a change of an evaluation was refused or failed.
#[derive(Debug)]
pub enum EvaluationError {
    /// The change does not hold together; holds the reason (an evaluation
    /// without a product, a document of another plan, a file of another
    /// kind).
    Refused(String),
    /// The text of a PDF could not be read.
    Pdf(PdfError),
    /// The way to the AI refused a question (no key, a question that does
    /// not fit even with the passages of its documents).
    Ai(AiError),
    /// A database failed, or refused what was given to it.
    Db(DbError),
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationError::Refused(reason) => formatter.write_str(reason),
            EvaluationError::Pdf(error) => write!(formatter, "{error}"),
            EvaluationError::Ai(error) => write!(formatter, "{error}"),
            EvaluationError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EvaluationError {}

impl From<PdfError> for EvaluationError {
    fn from(error: PdfError) -> EvaluationError {
        EvaluationError::Pdf(error)
    }
}

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
