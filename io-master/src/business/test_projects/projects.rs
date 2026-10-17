//! The test projects as a resource (steps 25 and 39 of `docs/todo.md`,
//! operations named in step 40). A project open for editing is an
//! object in memory (`workspace.rs`); every change is written through to
//! the database at once (`db/test_projects/`), then made in memory.
//!
//! ```text
//! list()        GET     /projects
//! get()         GET     /projects/{id}              without its schemes
//! existing()    GET     /projects/{id}              or the error that it does not exist
//! data()        GET     /projects/{id}/data         with schemes, tiles and links, as stored
//! create()      POST    /projects
//! update()      PUT     /projects/{id}              fields and documents of the open project
//! set_ready()   PUT     /projects/{id}/ready
//! delete()      DELETE  /projects/{id}              with everything of it; closed in memory
//! ```

use crate::business::standards;
use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::workspace::{OpenProject, ProjectWorkspace};
use crate::db::error::DbError;
use crate::db::test_projects as db_test_projects;
use crate::db::user::database::UserDb;
use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{Project, ProjectData, ProjectFields};

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// Every project, by name.
pub fn list(db: &UserDb) -> Result<Vec<Project>, DbError> {
    db_test_projects::read::projects(db)
}

/// One project, without its schemes; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Project>, DbError> {
    db_test_projects::read::project_by_id(db, id)
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
        schemes: db_test_projects::read::schemes(db, id)?,
        tiles: db_test_projects::read::project_tiles(db, id)?,
        links: db_test_projects::read::links(db, id)?,
        project,
    }))
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

/// Checks the fields (normalized) and the documents of a project, and
/// its name against the other projects (`id`: the project itself, when it is
/// changed).
pub(super) fn check(db: &UserDb, id: Option<i64>, fields: &ProjectFields) -> Result<(), StoreError> {
    fields.check()?;

    for document in &fields.documents {
        if standards::documents::get(db, *document)?.is_none() {
            return Err(SchemeError::NotFound("document").into());
        }
    }

    let taken = db_test_projects::read::project_named(db, &fields.name)?;

    if taken.is_some_and(|other| Some(other) != id) {
        return Err(SchemeError::NameTaken.into());
    }

    Ok(())
}

/// Makes a project; returns its number.
pub fn create(db: &UserDb, fields: &ProjectFields) -> Result<i64, StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    check(db, None, fields)?;

    Ok(db_test_projects::create::project(db, fields)?)
}

/* ---------------------------------------------------------------
 * Update
 * ------------------------------------------------------------- */

/// Changes the fields of the open project. A document still quoted by
/// a tile, or holding the pages of a scheme, cannot be taken away.
pub fn update(db: &UserDb, open: &mut OpenProject, fields: &ProjectFields) -> Result<(), StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    let id = open.data().project.id;
    check(db, Some(id), fields)?;

    let data = open.data();
    let quoted = data.tiles.iter().flat_map(|tile| tile.fields.sources.iter().map(|source| source.document_id));
    let paged = data.schemes.iter().filter_map(|scheme| scheme.fields.pages.map(|pages| pages.document_id));
    let used: Vec<i64> = quoted.chain(paged).collect();

    if used.iter().any(|document| !fields.documents.contains(document)) {
        return Err(SchemeError::Invalid(
            "documents",
            "a tile quotes, or a scheme covers pages of, a document taken away from the project; change those first"
                .to_owned(),
        )
        .into());
    }

    db_test_projects::update::project(db, id, fields)?;

    let project = &mut open.data_mut().project;
    project.fields = fields.clone();

    Ok(())
}

/// Marks the open project ready, or a draft again.
pub fn set_ready(db: &UserDb, open: &mut OpenProject, ready: bool) -> Result<(), StoreError> {
    db_test_projects::update::ready(db, open.data().project.id, ready)?;
    open.data_mut().project.ready = ready;

    Ok(())
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Deletes project `id` and closes it; returns whether it existed.
pub fn delete(db: &UserDb, workspace: &mut ProjectWorkspace, id: i64) -> Result<bool, DbError> {
    let deleted = db_test_projects::delete::project(db, id)?;
    workspace.close(id);

    Ok(deleted)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
