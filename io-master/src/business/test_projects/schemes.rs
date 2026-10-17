//! The schemes (chapters) of an open test project as a resource (step 25
//! of `docs/todo.md`, operations named in step 40): checked on the
//! project in memory, written through to the database, then made in
//! memory.
//!
//! ```text
//! get()        GET     /schemes/{id}
//! existing()   GET     /schemes/{id}     or the error that it does not exist
//! create()     POST    /projects/{id}/schemes
//! update()     PUT     /schemes/{id}
//! accept()     PUT     /schemes/{id}/accepted
//! delete()     DELETE  /schemes/{id}     with its tiles and their links; "Undo" forgets
//! project_of() GET     /schemes/{id}/project   as stored, before the project is open
//! accept_many(), delete_many()   the same for several schemes
//! ```

use crate::business::standards;
use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::workspace::OpenProject;
use crate::db::error::DbError;
use crate::db::test_projects as db_test_projects;
use crate::db::user::database::UserDb;
use crate::model::test_projects::rules::ProjectError;
use crate::model::test_projects::{ProposalState, Scheme, SchemeFields};
use crate::model::text::{Entity, Field};

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// Scheme `id` of the open project; `None` when it has no such.
pub fn get(open: &OpenProject, id: i64) -> Option<&Scheme> {
    open.data().schemes.iter().find(|scheme| scheme.id == id)
}

/// Scheme `id` of the open project, or the error that it has no such.
pub fn existing(open: &OpenProject, id: i64) -> Result<&Scheme, StoreError> {
    get(open, id).ok_or(StoreError::Rule(ProjectError::NotFound(Entity::Scheme)))
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

/// Checks the fields of a scheme of the open project: its pages in a
/// document of the project, its number not used by another scheme of
/// it (`id`: the scheme itself, when it is changed).
pub(super) fn check(db: &UserDb, open: &OpenProject, id: Option<i64>, fields: &SchemeFields) -> Result<(), StoreError> {
    fields.check()?;

    let data = open.data();

    if let Some(pages) = fields.pages {
        if !data.project.fields.documents.contains(&pages.document_id) {
            return Err(ProjectError::Invalid(
                Field::Pages,
                "the pages of a scheme must be in a document of its project".to_owned(),
            )
            .into());
        }

        let count = standards::documents::get(db, pages.document_id)?.map_or(0, |document| document.page_count);

        if pages.last_page > count {
            return Err(ProjectError::Invalid(Field::Pages, format!("the document has only {count} pages")).into());
        }
    }

    let number = fields.number.trim();

    if !number.is_empty()
        && data
            .schemes
            .iter()
            .any(|other| Some(other.id) != id && other.fields.number.trim().eq_ignore_ascii_case(number))
    {
        return Err(ProjectError::Invalid(
            Field::Number,
            format!("another scheme of the project has the number {number}"),
        )
        .into());
    }

    Ok(())
}

/// Adds a scheme at the end of the list of the open project; returns
/// its number.
pub fn create(db: &UserDb, open: &mut OpenProject, fields: &SchemeFields) -> Result<i64, StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    check(db, open, None, fields)?;

    let project = open.data().project.id;
    let id = db_test_projects::create::scheme(db, project, fields)?;

    open.data_mut().schemes.push(Scheme { id, project, fields: fields.clone() });

    Ok(id)
}

/* ---------------------------------------------------------------
 * Update
 * ------------------------------------------------------------- */

/// Changes the fields of scheme `id`.
pub fn update(db: &UserDb, open: &mut OpenProject, id: i64, fields: &SchemeFields) -> Result<(), StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    existing(open, id)?;
    check(db, open, Some(id), fields)?;

    db_test_projects::update::scheme(db, id, fields)?;

    if let Some(scheme) = open.data_mut().schemes.iter_mut().find(|scheme| scheme.id == id) {
        scheme.fields = fields.clone();
    }

    Ok(())
}

/// Accepts a scheme proposed by the AI.
pub fn accept(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<(), StoreError> {
    existing(open, id)?;

    if !db_test_projects::update::accept_scheme(db, id)? {
        return Err(ProjectError::NotFound(Entity::Scheme).into());
    }

    if let Some(scheme) = open.data_mut().schemes.iter_mut().find(|scheme| scheme.id == id) {
        scheme.fields.state = ProposalState::Accepted;
    }

    Ok(())
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Deletes scheme `id`; returns whether it existed.
pub fn delete(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<bool, DbError> {
    // Only a scheme of this project.
    if !open.data().schemes.iter().any(|scheme| scheme.id == id) {
        return Ok(false);
    }

    let deleted = db_test_projects::delete::scheme(db, id)?;
    let data = open.data_mut();
    let gone: Vec<i64> = data.tiles.iter().filter(|tile| tile.scheme == id).map(|tile| tile.id).collect();

    data.schemes.retain(|scheme| scheme.id != id);
    data.tiles.retain(|tile| tile.scheme != id);
    data.links.retain(|link| !gone.contains(&link.from) && !gone.contains(&link.to));
    open.forget_versions();

    Ok(deleted)
}

/* ---------------------------------------------------------------
 * Several schemes, and the project of a scheme
 * ------------------------------------------------------------- */

/// The project scheme `id` belongs to, as stored (to open the project
/// before the scheme is worked on); `None` when there is no such.
pub fn project_of(db: &UserDb, id: i64) -> Result<Option<i64>, DbError> {
    Ok(db_test_projects::read::scheme(db, id)?.map(|scheme| scheme.project))
}

/// Accepts the schemes `ids` of the open project; a number that is not
/// a scheme of it is left out.
pub fn accept_many(db: &UserDb, open: &mut OpenProject, ids: &[i64]) -> Result<(), StoreError> {
    for id in ids {
        if get(open, *id).is_some() {
            accept(db, open, *id)?;
        }
    }

    Ok(())
}

/// Deletes the schemes `ids` of the open project with their tiles.
pub fn delete_many(db: &UserDb, open: &mut OpenProject, ids: &[i64]) -> Result<(), DbError> {
    for id in ids {
        delete(db, open, *id)?;
    }

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "schemes_tests.rs"]
mod tests;
