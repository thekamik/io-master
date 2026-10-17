//! Deleting a scheme of an open test project with its tiles and their
//! links (also the links to tiles of other schemes). Its tiles cannot
//! come back without it, so the versions for "Undo" are forgotten.

use crate::db::error::DbError;
use crate::db::test_projects::delete as store;
use crate::db::user::database::UserDb;

use crate::test_projects::workspace::OpenProject;

/// Deletes scheme `id`; returns whether it existed.
pub fn scheme(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<bool, DbError> {
    // Only a scheme of this project.
    if !open.data().schemes.iter().any(|scheme| scheme.id == id) {
        return Ok(false);
    }

    let deleted = store::scheme(db, id)?;
    let data = open.data_mut();
    let gone: Vec<i64> = data.tiles.iter().filter(|tile| tile.scheme == id).map(|tile| tile.id).collect();

    data.schemes.retain(|scheme| scheme.id != id);
    data.tiles.retain(|tile| tile.scheme != id);
    data.links.retain(|link| !gone.contains(&link.from) && !gone.contains(&link.to));
    open.forget_versions();

    Ok(deleted)
}
