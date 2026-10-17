//! Deleting test projects, schemes, tiles and links, each with what
//! hangs on it, in one transaction.

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes a project with its schemes, tiles and links; returns
/// whether it existed.
pub fn project(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM tile_sources WHERE tile_id IN \
         (SELECT t.id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM links WHERE project_id = ?1", [id])?;
    transaction.execute(
        "DELETE FROM tiles WHERE scheme_id IN (SELECT id FROM schemes WHERE project_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM schemes WHERE project_id = ?1", [id])?;
    transaction.execute("DELETE FROM project_documents WHERE project_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM projects WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes a scheme with its tiles and their links (also the links to
/// tiles of other schemes); returns whether it existed.
pub fn scheme(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM tile_sources WHERE tile_id IN (SELECT id FROM tiles WHERE scheme_id = ?1)",
        [id],
    )?;
    transaction.execute(
        "DELETE FROM links WHERE from_tile IN (SELECT id FROM tiles WHERE scheme_id = ?1) \
         OR to_tile IN (SELECT id FROM tiles WHERE scheme_id = ?1)",
        [id],
    )?;
    transaction.execute("DELETE FROM tiles WHERE scheme_id = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM schemes WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes a tile with its sources and links; returns whether it
/// existed.
pub fn tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM tile_sources WHERE tile_id = ?1", [id])?;
    transaction.execute("DELETE FROM links WHERE from_tile = ?1 OR to_tile = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM tiles WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Removes a link; returns whether it existed.
pub fn link(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM links WHERE id = ?1", [id])? > 0)
}
