//! Removing a link of an open test project.

use crate::db::error::DbError;
use crate::db::test_projects::delete as store;
use crate::db::user::database::UserDb;

use crate::test_projects::workspace::OpenProject;

/// Removes link `id`; returns whether it existed (in the project).
pub fn link(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<bool, DbError> {
    if super::read::get(open, id).is_none() {
        return Ok(false);
    }

    let deleted = store::link(db, id)?;
    open.data_mut().links.retain(|link| link.id != id);

    Ok(deleted)
}
