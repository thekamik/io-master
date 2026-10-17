//! Reading test projects as they are stored: the list, one project,
//! and a whole project (to open it, to export it, to import it into an
//! evaluation). An open project is read from memory instead
//! (`workspace.rs`, `OpenProject::data`).

use crate::db::error::DbError;
use crate::db::test_projects::read as stored;
use crate::db::user::database::UserDb;

use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{Project, ProjectData};
use crate::test_projects::error::StoreError;

/// Every project, by name.
pub fn list(db: &UserDb) -> Result<Vec<Project>, DbError> {
    stored::projects(db)
}

/// One project, without its schemes; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Project>, DbError> {
    stored::project_by_id(db, id)
}

/// The project, or the error that it does not exist.
pub fn existing(db: &UserDb, id: i64) -> Result<Project, StoreError> {
    get(db, id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("project")))
}

/// A project with its schemes, tiles and links, as stored.
pub fn data(db: &UserDb, id: i64) -> Result<Option<ProjectData>, DbError> {
    let Some(project) = get(db, id)? else {
        return Ok(None);
    };

    Ok(Some(ProjectData {
        schemes: stored::schemes(db, id)?,
        tiles: stored::project_tiles(db, id)?,
        links: stored::links(db, id)?,
        project,
    }))
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, update, delete).
#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
