//! Deleting a test project with its schemes, tiles and links; it is
//! closed in memory as well.

use crate::db::error::DbError;
use crate::db::test_projects::delete as store;
use crate::db::user::database::UserDb;

use crate::business::test_projects::workspace::Workspace;

/// Deletes project `id` and closes it; returns whether it existed.
pub fn project(db: &UserDb, workspace: &mut Workspace, id: i64) -> Result<bool, DbError> {
    let deleted = store::project(db, id)?;
    workspace.close(id);

    Ok(deleted)
}
