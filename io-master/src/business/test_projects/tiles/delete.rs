//! Deleting a tile of an open test project with its sources and links.

use crate::db::error::DbError;
use crate::db::test_projects::delete as store;
use crate::db::user::database::UserDb;

use crate::business::test_projects::workspace::OpenProject;

/// Deletes tile `id`; returns whether it existed (in the project).
pub fn tile(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<bool, DbError> {
    if super::read::get(open, id).is_none() {
        return Ok(false);
    }

    let deleted = store::tile(db, id)?;
    let data = open.data_mut();

    data.tiles.retain(|tile| tile.id != id);
    data.links.retain(|link| link.from != id && link.to != id);

    Ok(deleted)
}
