//! The error of the panel of the agents: a change refused, a file of a
//! source that cannot be read, a failure of the database. The commands of
//! the panel tell its kinds apart for the page.

use std::fmt;

use crate::db::error::DbError;
use crate::pdf::poppler::PdfError;

/// Why a change of an agent or of its knowledge was refused or failed.
#[derive(Debug)]
pub enum AgentError {
    /// The change does not hold together; holds the reason (an agent
    /// without a name, a limit out of its range, a file of another kind).
    Refused(String),
    /// The text of a PDF could not be read.
    Pdf(PdfError),
    /// A database failed, or refused what was given to it.
    Db(DbError),
}

impl fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::Refused(reason) => formatter.write_str(reason),
            AgentError::Pdf(error) => write!(formatter, "{error}"),
            AgentError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AgentError {}

impl From<PdfError> for AgentError {
    fn from(error: PdfError) -> AgentError {
        AgentError::Pdf(error)
    }
}

impl From<DbError> for AgentError {
    fn from(error: DbError) -> AgentError {
        AgentError::Db(error)
    }
}
