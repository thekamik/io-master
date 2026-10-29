//! The error of the making of a plan: a document that is no catalog, a
//! change the plan does not allow, a failure of the database. The commands
//! of the panel tell its kinds apart for the page.

use std::fmt;

use crate::db::error::DbError;
use crate::panels::logic::evaluation_plan::oscal::OscalError;

/// Why a step of the making of a plan was refused or failed.
#[derive(Debug)]
pub enum PlanError {
    /// The document of a standard is no catalog the program takes.
    Oscal(OscalError),
    /// The change does not hold together with the plan; holds the reason
    /// (a plan no more in planning, a standard still used by a work unit).
    Refused(String),
    /// A database failed, or refused what was given to it.
    Db(DbError),
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::Oscal(error) => write!(formatter, "{error}"),
            PlanError::Refused(reason) => formatter.write_str(reason),
            PlanError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for PlanError {}

impl From<OscalError> for PlanError {
    fn from(error: OscalError) -> PlanError {
        PlanError::Oscal(error)
    }
}

impl From<DbError> for PlanError {
    fn from(error: DbError) -> PlanError {
        PlanError::Db(error)
    }
}
