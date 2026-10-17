//! Reading the links of an open test project, from memory.

use crate::model::test_projects::Link;
use crate::test_projects::workspace::OpenProject;

/// Link `id` of the open project; `None` when it has no such.
pub fn get(open: &OpenProject, id: i64) -> Option<&Link> {
    open.data().links.iter().find(|link| link.id == id)
}

/// The links that join tile `tile` with another.
pub fn of_tile(open: &OpenProject, tile: i64) -> Vec<&Link> {
    open.data().links.iter().filter(|link| link.from == tile || link.to == tile).collect()
}
