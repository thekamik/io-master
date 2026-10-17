//! Reading the schemes of an open test project, from memory.

use crate::model::test_projects::Scheme;
use crate::model::test_projects::rules::SchemeError;
use crate::test_projects::error::StoreError;
use crate::test_projects::workspace::OpenProject;

/// Scheme `id` of the open project; `None` when it has no such.
pub fn get(open: &OpenProject, id: i64) -> Option<&Scheme> {
    open.data().schemes.iter().find(|scheme| scheme.id == id)
}

/// Scheme `id` of the open project, or the error that it has no such.
pub fn existing(open: &OpenProject, id: i64) -> Result<&Scheme, StoreError> {
    get(open, id).ok_or(StoreError::Scheme(SchemeError::NotFound("scheme")))
}

// Unit tests, in their own file to keep this one readable. They
// cover the folders schemes/, tiles/ and links/ and the workspace.
#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
