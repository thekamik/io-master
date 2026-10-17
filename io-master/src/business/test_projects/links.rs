//! The links between the tiles of an open test project as a resource
//! (steps 19 and 26 of `docs/todo.md`, operations named in step 40), by
//! the rules of `model/test_projects/rules.rs`, checked against all the
//! links of the project in memory; written through, then made in memory.
//!
//! ```text
//! get()       GET     /links/{id}
//! of_tile()   GET     /tiles/{id}/links
//! create()    POST    /links
//! delete()    DELETE  /links/{id}
//! ```

use crate::business::test_projects::error::StoreError;
use crate::business::test_projects::workspace::OpenProject;
use crate::db::error::DbError;
use crate::db::test_projects as db_test_projects;
use crate::db::user::database::UserDb;
use crate::model::test_projects::rules::{SchemeError, check_link};
use crate::model::test_projects::{Link, TileKind};

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// Link `id` of the open project; `None` when it has no such.
pub fn get(open: &OpenProject, id: i64) -> Option<&Link> {
    open.data().links.iter().find(|link| link.id == id)
}

/// The links that join tile `tile` with another.
pub fn of_tile(open: &OpenProject, tile: i64) -> Vec<&Link> {
    open.data().links.iter().filter(|link| link.from == tile || link.to == tile).collect()
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

/// Links tile `from` to tile `to`; returns the number of the link.
pub fn create(db: &UserDb, open: &mut OpenProject, from: i64, to: i64) -> Result<i64, StoreError> {
    let data = open.data();
    let kinds: Vec<(i64, TileKind)> = data.tiles.iter().map(|tile| (tile.id, tile.fields.kind)).collect();

    if !kinds.iter().any(|(id, _)| *id == from) || !kinds.iter().any(|(id, _)| *id == to) {
        return Err(SchemeError::Invalid("link", "only tiles of one project can be linked".to_owned()).into());
    }

    check_link(&kinds, &data.links, from, to)?;

    let project = data.project.id;
    let id = db_test_projects::create::link(db, project, from, to)?;
    open.data_mut().links.push(Link { id, from, to });

    Ok(id)
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Removes link `id`; returns whether it existed (in the project).
pub fn delete(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<bool, DbError> {
    if get(open, id).is_none() {
        return Ok(false);
    }

    let deleted = db_test_projects::delete::link(db, id)?;
    open.data_mut().links.retain(|link| link.id != id);

    Ok(deleted)
}
