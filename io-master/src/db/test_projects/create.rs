//! Creating test projects, schemes, tiles and links. The rules of
//! `model/test_projects.rs` are checked by the callers
//! (`test_projects/projects/`, `test_projects/`) before.

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::test_projects::{ProjectFields, SchemeFields, Source, TileFields};

use super::rows::{INSERT_TILE, write_documents, write_scheme, write_sources, write_tile};

/// Stores a new project (not ready) with its documents, in one
/// transaction; returns its number.
pub fn project(db: &UserDb, fields: &ProjectFields) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO projects (name, version, description, author, ready) VALUES (?1, ?2, ?3, ?4, 0)",
        params![fields.name.trim(), fields.version.trim(), fields.description, fields.author.trim()],
    )?;

    let id = transaction.last_insert_rowid();
    write_documents(&transaction, id, &fields.documents)?;
    transaction.commit()?;

    Ok(id)
}

/// Stores a scheme at the end of the list of `project`; returns its
/// number.
pub fn scheme(db: &UserDb, project: i64, fields: &SchemeFields) -> Result<i64, DbError> {
    write_scheme(
        db.connection(),
        "INSERT INTO schemes (project_id, number, name, state, document_id, first_page, last_page, position) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, \
         (SELECT COALESCE(MAX(position), 0) + 1 FROM schemes WHERE project_id = ?1))",
        project,
        fields,
    )?;

    Ok(db.connection().last_insert_rowid())
}

/// Stores a tile of `scheme` with its `sources` (already checked on
/// their pages), in one transaction; returns its number.
pub fn tile(db: &UserDb, scheme: i64, fields: &TileFields, sources: &[Source]) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    write_tile(&transaction, INSERT_TILE, scheme, fields)?;

    let id = transaction.last_insert_rowid();
    write_sources(&transaction, id, sources)?;
    transaction.commit()?;

    Ok(id)
}

/// Stores a link of `project`; returns its number.
pub fn link(db: &UserDb, project: i64, from: i64, to: i64) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO links (project_id, from_tile, to_tile) VALUES (?1, ?2, ?3)",
        params![project, from, to],
    )?;

    Ok(db.connection().last_insert_rowid())
}
