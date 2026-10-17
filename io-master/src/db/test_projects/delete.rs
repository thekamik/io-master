//! Deleting test projects, schemes, tiles and links, each with what
//! hangs on it, in one transaction.

use rusqlite::{Connection, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;

/// Deletes a project with its schemes, tiles and links; returns
/// whether it existed.
pub fn project(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    content(&transaction, id)?;
    transaction.execute("DELETE FROM schemes WHERE project_id = :id", named_params! { ":id": id })?;
    transaction.execute("DELETE FROM project_documents WHERE project_id = :id", named_params! { ":id": id })?;
    let deleted = transaction.execute("DELETE FROM projects WHERE id = :id", named_params! { ":id": id })? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes the content of `project`: the tiles of all its schemes, their
/// sources, and its links; the schemes stay. Inside the transaction of
/// the caller (`project` here; `update.rs` when the content is
/// replaced).
pub(super) fn content(connection: &Connection, project: i64) -> rusqlite::Result<()> {
    let tiles = "SELECT t.id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = :project";

    connection.execute(
        &format!("DELETE FROM tile_sources WHERE tile_id IN ({tiles})"),
        named_params! { ":project": project },
    )?;
    connection.execute("DELETE FROM links WHERE project_id = :project", named_params! { ":project": project })?;
    connection.execute(
        "DELETE FROM tiles WHERE scheme_id IN (SELECT id FROM schemes WHERE project_id = :project)",
        named_params! { ":project": project },
    )?;

    Ok(())
}

/// Deletes a scheme with its tiles and their links (also the links to
/// tiles of other schemes); returns whether it existed.
pub fn scheme(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;
    let tiles = "SELECT id FROM tiles WHERE scheme_id = :id";

    transaction
        .execute(&format!("DELETE FROM tile_sources WHERE tile_id IN ({tiles})"), named_params! { ":id": id })?;
    transaction.execute(
        &format!("DELETE FROM links WHERE from_tile IN ({tiles}) OR to_tile IN ({tiles})"),
        named_params! { ":id": id },
    )?;
    transaction.execute("DELETE FROM tiles WHERE scheme_id = :id", named_params! { ":id": id })?;
    let deleted = transaction.execute("DELETE FROM schemes WHERE id = :id", named_params! { ":id": id })? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Deletes a tile with its sources and links; returns whether it
/// existed.
pub fn tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM tile_sources WHERE tile_id = :id", named_params! { ":id": id })?;
    transaction.execute("DELETE FROM links WHERE from_tile = :id OR to_tile = :id", named_params! { ":id": id })?;
    let deleted = transaction.execute("DELETE FROM tiles WHERE id = :id", named_params! { ":id": id })? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/// Removes a link; returns whether it existed.
pub fn link(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM links WHERE id = :id", named_params! { ":id": id })? > 0)
}
