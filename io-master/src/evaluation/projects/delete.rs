//! Deleting an evaluation with its structure, results, proposals and
//! journal (overwritten in the file: `db/evaluations/delete.rs`); it is
//! closed in memory as well.

use crate::db::error::DbError;
use crate::db::evaluations::delete as remove;
use crate::db::user::database::UserDb;

use crate::evaluation::workspace::Workspace;

/// Deletes evaluation `id` and closes it; returns whether it existed.
pub fn evaluation(db: &UserDb, workspace: &mut Workspace, id: i64) -> Result<bool, DbError> {
    let deleted = remove::evaluation(db, id)?;
    workspace.close(id);

    Ok(deleted)
}
