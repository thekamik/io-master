//! Reading the tiles of an open test project, from memory.

use crate::test_projects::model::Tile;
use crate::test_projects::rules::{SchemeError, StoreError};
use crate::test_projects::workspace::OpenProject;

/// Tile `id` of the open project; `None` when it has no such.
pub fn get(open: &OpenProject, id: i64) -> Option<&Tile> {
    open.data().tiles.iter().find(|tile| tile.id == id)
}

/// Tile `id` of the open project, or the error that it has no such.
pub fn existing(open: &OpenProject, id: i64) -> Result<&Tile, StoreError> {
    get(open, id).ok_or(StoreError::Scheme(SchemeError::NotFound("tile")))
}

/// The tiles of scheme `scheme`, in the order they were made.
pub fn of_scheme(open: &OpenProject, scheme: i64) -> Vec<&Tile> {
    open.data().tiles.iter().filter(|tile| tile.scheme == scheme).collect()
}
