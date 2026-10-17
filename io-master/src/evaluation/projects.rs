//! The test projects of an evaluator, kept in their encrypted database
//! (`db/user/`):
//!
//! ```text
//! projects           name, version, description, author, ready
//! project_documents  the documents of the standard a project uses
//! ```
//!
//! The schemes of a project, their tiles and the links are stored by
//! `schemes.rs`; [`data`] reads a whole project at once. Every change
//! is checked by the rules of `scheme.rs` before it is stored; changes
//! of several rows are made in one transaction, so a project is never
//! left half changed. The SQL is in `db/test_projects/`.

use crate::db::error::DbError;
use crate::db::test_projects::{create as store, delete as remove, read, update as change};
use crate::db::user::database::UserDb;

use super::documents;
use super::scheme::{Link, Project, ProjectFields, Scheme, SchemeError, Tile};
use super::schemes::{self, StoreError};

/// A project with everything in it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectData {
    pub project: Project,
    /// In the order of the list of the project.
    pub schemes: Vec<Scheme>,
    /// The tiles of all its schemes.
    pub tiles: Vec<Tile>,
    pub links: Vec<Link>,
}

/// Checks the fields and the documents of a project, and the name
/// against the other projects.
fn check_project(db: &UserDb, id: Option<i64>, fields: &ProjectFields) -> Result<(), StoreError> {
    fields.check()?;

    for document in &fields.documents {
        if documents::get(db, *document)?.is_none() {
            return Err(SchemeError::NotFound("document").into());
        }
    }

    let taken = read::project_named(db, fields.name.trim())?;

    if taken.is_some_and(|other| Some(other) != id) {
        return Err(SchemeError::NameTaken.into());
    }

    Ok(())
}

/// Makes a project; returns its number.
pub fn create(db: &UserDb, fields: &ProjectFields) -> Result<i64, StoreError> {
    check_project(db, None, fields)?;

    Ok(store::project(db, fields)?)
}

/// Every project, by name.
pub fn list(db: &UserDb) -> Result<Vec<Project>, DbError> {
    read::projects(db)
}

/// One project, without its schemes; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Project>, DbError> {
    read::project_by_id(db, id)
}

/// The project, or the error that it does not exist.
pub fn existing(db: &UserDb, id: i64) -> Result<Project, StoreError> {
    get(db, id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("project")))
}

/// Changes the fields of a project. A document still quoted by a tile,
/// or holding the pages of a scheme, cannot be taken away.
pub fn update(db: &UserDb, id: i64, fields: &ProjectFields) -> Result<(), StoreError> {
    existing(db, id)?;
    check_project(db, Some(id), fields)?;

    let used = read::documents_in_use(db, id)?;

    if used.iter().any(|document| !fields.documents.contains(document)) {
        return Err(SchemeError::Invalid(
            "documents",
            "a tile quotes, or a scheme covers pages of, a document taken away from the project; change those first"
                .to_owned(),
        )
        .into());
    }

    Ok(change::project(db, id, fields)?)
}

/// Marks a project ready, or a draft again.
pub fn set_ready(db: &UserDb, id: i64, ready: bool) -> Result<(), StoreError> {
    existing(db, id)?;

    Ok(change::ready(db, id, ready)?)
}

/// Deletes a project with its schemes, tiles and links; returns
/// whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    remove::project(db, id)
}

/// A project with its schemes, tiles and links.
pub fn data(db: &UserDb, id: i64) -> Result<Option<ProjectData>, DbError> {
    let Some(project) = get(db, id)? else {
        return Ok(None);
    };

    Ok(Some(ProjectData {
        schemes: schemes::of_project(db, id)?,
        tiles: schemes::project_tiles(db, id)?,
        links: schemes::links(db, id)?,
        project,
    }))
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
