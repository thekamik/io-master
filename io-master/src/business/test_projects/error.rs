//! The error of a change of a test project: a rule of the project
//! broken (`model/test_projects/rules.rs`), or the database failed.

use std::fmt;

use crate::db::error::DbError;
use crate::model::test_projects::rules::ProjectError;

/// Why a change of a project was not made.
#[derive(Debug)]
pub enum StoreError {
    /// It breaks a rule of a project.
    Rule(ProjectError),
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Rule(error) => write!(formatter, "{error}"),
            StoreError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<ProjectError> for StoreError {
    fn from(error: ProjectError) -> StoreError {
        StoreError::Rule(error)
    }
}

impl From<DbError> for StoreError {
    fn from(error: DbError) -> StoreError {
        StoreError::Db(error)
    }
}
