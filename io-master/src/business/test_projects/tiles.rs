//! The tiles of the schemes of an open test project as a resource (steps
//! 19 and 26 of `docs/todo.md`, operations named in step 40): checked on
//! the project in memory, their quotes looked for on their pages, written
//! through to the database, then made in memory.
//!
//! ```text
//! get()              GET     /tiles/{id}
//! existing()         GET     /tiles/{id}           or the error that it does not exist
//! list()             GET     /schemes/{id}/tiles
//! create()           POST    /schemes/{id}/tiles
//! update()           PUT     /tiles/{id}
//! set_position()     PUT     /tiles/{id}/position  on the canvas
//! accept()           PUT     /tiles/{id}/accepted
//! move_to_scheme()   PUT     /tiles/scheme         to another scheme of the project
//! delete()           DELETE  /tiles/{id}           with its sources and links
//! ```

use crate::business::standards;
use crate::business::standards::quote::is_on_page;
use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::schemes::existing as existing_scheme;
use crate::business::test_projects::workspace::OpenProject;
use crate::db::error::DbError;
use crate::db::test_projects as db_test_projects;
use crate::db::user::database::UserDb;
use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{ProposalState, Source, Tile, TileFields};

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// Tile `id` of the open project; `None` when it has no such.
pub fn get(open: &OpenProject, id: i64) -> Option<&Tile> {
    open.data().tiles.iter().find(|tile| tile.id == id)
}

/// Tile `id` of the open project, or the error that it has no such.
pub fn existing(open: &OpenProject, id: i64) -> Result<&Tile, StoreError> {
    get(open, id).ok_or(StoreError::Scheme(SchemeError::NotFound("tile")))
}

/// The tiles of scheme `scheme`, in the order they were made.
pub fn list(open: &OpenProject, scheme: i64) -> Vec<&Tile> {
    open.data().tiles.iter().filter(|tile| tile.scheme == scheme).collect()
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

/// Checks a tile for the open project, and finds its quotes on their
/// pages: the sources returned carry `found`, set by the program (a
/// passage the AI made up stays visible as such).
pub(super) fn checked_sources(db: &UserDb, open: &OpenProject, fields: &TileFields) -> Result<Vec<Source>, StoreError> {
    fields.check()?;

    let documents = &open.data().project.fields.documents;
    let mut checked = Vec::with_capacity(fields.sources.len());

    for source in &fields.sources {
        if !documents.contains(&source.document_id) {
            return Err(SchemeError::Invalid(
                "sources",
                "a source quotes a document the project does not use; add it to the project first".to_owned(),
            )
            .into());
        }

        let text = standards::documents::page(db, source.document_id, source.page)?
            .ok_or_else(|| SchemeError::Invalid("sources", format!("the document has no page {}", source.page)))?;

        checked.push(Source { found: is_on_page(&source.quote, &text), ..source.clone() });
    }

    Ok(checked)
}

/// Adds a tile to scheme `scheme` of the open project; returns its
/// number.
pub fn create(db: &UserDb, open: &mut OpenProject, scheme: i64, fields: &TileFields) -> Result<i64, StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    existing_scheme(open, scheme)?;
    let sources = checked_sources(db, open, fields)?;

    let id = db_test_projects::create::tile(db, scheme, fields, &sources)?;

    open.data_mut().tiles.push(Tile { id, scheme, fields: TileFields { sources, ..fields.clone() } });

    Ok(id)
}

/* ---------------------------------------------------------------
 * Update
 * ------------------------------------------------------------- */

/// Changes tile `id`. Its kind can be changed only while no link joins
/// it: a link is allowed by the kinds it joins.
pub fn update(db: &UserDb, open: &mut OpenProject, id: i64, fields: &TileFields) -> Result<(), StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    let old_kind = existing(open, id)?.fields.kind;
    let sources = checked_sources(db, open, fields)?;

    if old_kind != fields.kind && open.data().links.iter().any(|link| link.from == id || link.to == id) {
        return Err(SchemeError::Invalid(
            "kind",
            "the kind of a linked tile cannot be changed; remove its links first".to_owned(),
        )
        .into());
    }

    db_test_projects::update::tile(db, id, fields, &sources)?;

    if let Some(tile) = open.data_mut().tiles.iter_mut().find(|tile| tile.id == id) {
        tile.fields = TileFields { sources, ..fields.clone() };
    }

    Ok(())
}

/// Moves tile `id` on the canvas.
pub fn set_position(db: &UserDb, open: &mut OpenProject, id: i64, x: f64, y: f64) -> Result<(), StoreError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(SchemeError::Invalid("position", "the position is not a number".to_owned()).into());
    }

    existing(open, id)?;

    if !db_test_projects::update::tile_position(db, id, x, y)? {
        return Err(SchemeError::NotFound("tile").into());
    }

    if let Some(tile) = open.data_mut().tiles.iter_mut().find(|tile| tile.id == id) {
        tile.fields.x = x;
        tile.fields.y = y;
    }

    Ok(())
}

/// Accepts tile `id`, proposed by the AI.
pub fn accept(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<(), StoreError> {
    existing(open, id)?;

    if !db_test_projects::update::accept_tile(db, id)? {
        return Err(SchemeError::NotFound("tile").into());
    }

    if let Some(tile) = open.data_mut().tiles.iter_mut().find(|tile| tile.id == id) {
        tile.fields.state = ProposalState::Accepted;
    }

    Ok(())
}

/// Moves tiles to another scheme of the open project (step 26); their
/// links stay, they belong to the project. Nothing is moved when one of
/// them is not a tile of the project.
pub fn move_to_scheme(db: &UserDb, open: &mut OpenProject, ids: &[i64], scheme: i64) -> Result<(), StoreError> {
    existing_scheme(open, scheme)?;

    for id in ids {
        if get(open, *id).is_none() {
            return Err(SchemeError::Invalid(
                "scheme",
                "a tile can only be moved to a scheme of its own project".to_owned(),
            )
            .into());
        }
    }

    db_test_projects::update::tiles_scheme(db, ids, scheme)?;

    for tile in open.data_mut().tiles.iter_mut().filter(|tile| ids.contains(&tile.id)) {
        tile.scheme = scheme;
    }

    Ok(())
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Deletes tile `id`; returns whether it existed (in the project).
pub fn delete(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<bool, DbError> {
    if get(open, id).is_none() {
        return Ok(false);
    }

    let deleted = db_test_projects::delete::tile(db, id)?;
    let data = open.data_mut();

    data.tiles.retain(|tile| tile.id != id);
    data.links.retain(|link| link.from != id && link.to != id);

    Ok(deleted)
}
