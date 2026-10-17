//! The error of a change of a test project: a rule of the project
//! broken (`model/test_projects/rules.rs`), or the database failed.

use std::fmt;

use crate::db::error::DbError;
use crate::model::test_projects::rules::SchemeError;

/// Why a change of a project was not made.
#[derive(Debug)]
pub enum StoreError {
    /// It breaks a rule of a project.
    Scheme(SchemeError),
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Scheme(error) => write!(formatter, "{error}"),
            StoreError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<SchemeError> for StoreError {
    fn from(error: SchemeError) -> StoreError {
        StoreError::Scheme(error)
    }
}

impl From<DbError> for StoreError {
    fn from(error: DbError) -> StoreError {
        StoreError::Db(error)
    }
}
