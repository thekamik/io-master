//! Changing test projects, schemes and tiles. The rules of
//! `test_projects/model.rs` are checked by the callers before. A
//! function that changes a row that does not exist returns `false`.

use rusqlite::params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::test_projects::model::{Link, ProjectFields, ProposalState, SchemeFields, Source, Tile, TileFields};

use super::rows::{INSERT_TILE, write_documents, write_scheme, write_sources, write_tile};

/// Changes the fields and the documents of a project, in one
/// transaction.
pub fn project(db: &UserDb, id: i64, fields: &ProjectFields) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "UPDATE projects SET name = ?1, version = ?2, description = ?3, author = ?4 WHERE id = ?5",
        params![fields.name.trim(), fields.version.trim(), fields.description, fields.author.trim(), id],
    )?;
    write_documents(&transaction, id, &fields.documents)?;
    transaction.commit()?;

    Ok(())
}

/// Marks a project ready, or a draft again.
pub fn ready(db: &UserDb, id: i64, ready: bool) -> Result<(), DbError> {
    db.connection().execute("UPDATE projects SET ready = ?1 WHERE id = ?2", params![ready, id])?;

    Ok(())
}

/// Changes the fields of a scheme.
pub fn scheme(db: &UserDb, id: i64, fields: &SchemeFields) -> Result<(), DbError> {
    write_scheme(
        db.connection(),
        "UPDATE schemes SET number = ?2, name = ?3, state = ?4, document_id = ?5, first_page = ?6, last_page = ?7 \
         WHERE id = ?1",
        id,
        fields,
    )?;

    Ok(())
}

/// Marks a scheme proposed by the AI accepted.
pub fn accept_scheme(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "UPDATE schemes SET state = ?1 WHERE id = ?2",
        params![ProposalState::Accepted.name(), id],
    )? > 0)
}

/// Changes a tile and replaces its sources (already checked on their
/// pages), in one transaction.
pub fn tile(db: &UserDb, id: i64, fields: &TileFields, sources: &[Source]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    write_tile(
        &transaction,
        "UPDATE tiles SET kind = ?2, identifier = ?3, title = ?4, description = ?5, place = ?6, state = ?7, \
         external_document = ?8, external_place = ?9, sections = ?10, instructions = ?11, x = ?12, y = ?13 \
         WHERE id = ?1",
        id,
        fields,
    )?;
    write_sources(&transaction, id, sources)?;
    transaction.commit()?;

    Ok(())
}

/// Moves a tile on the canvas.
pub fn tile_position(db: &UserDb, id: i64, x: f64, y: f64) -> Result<bool, DbError> {
    Ok(db.connection().execute("UPDATE tiles SET x = ?1, y = ?2 WHERE id = ?3", params![x, y, id])? > 0)
}

/// Moves tiles to the scheme `scheme`, in one transaction.
pub fn tiles_scheme(db: &UserDb, ids: &[i64], scheme: i64) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    for id in ids {
        transaction.execute("UPDATE tiles SET scheme_id = ?1 WHERE id = ?2", params![scheme, id])?;
    }

    transaction.commit()?;

    Ok(())
}

/// Marks a tile proposed by the AI accepted.
pub fn accept_tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "UPDATE tiles SET state = ?1 WHERE id = ?2",
        params![ProposalState::Accepted.name(), id],
    )? > 0)
}

/// Replaces the tiles and links of `project` by `tiles` and `links`,
/// in one transaction. Every tile names its scheme; its sources are
/// stored as they come. The tiles get new numbers and the links follow
/// them; a link that names a tile not in `tiles` is skipped (the
/// caller has refused such links before). Returns the new number of
/// every old one.
pub fn replace_content(db: &UserDb, project: i64, tiles: &[Tile], links: &[Link]) -> Result<Vec<(i64, i64)>, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM tile_sources WHERE tile_id IN \
         (SELECT t.id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = ?1)",
        [project],
    )?;
    transaction.execute("DELETE FROM links WHERE project_id = ?1", [project])?;
    transaction.execute(
        "DELETE FROM tiles WHERE scheme_id IN (SELECT id FROM schemes WHERE project_id = ?1)",
        [project],
    )?;

    let mut numbers = Vec::with_capacity(tiles.len());

    for tile in tiles {
        write_tile(&transaction, INSERT_TILE, tile.scheme, &tile.fields)?;

        let id = transaction.last_insert_rowid();
        write_sources(&transaction, id, &tile.fields.sources)?;
        numbers.push((tile.id, id));
    }

    let new = |old: i64| numbers.iter().find(|(from, _)| *from == old).map(|(_, to)| *to);

    for link in links {
        if let (Some(from), Some(to)) = (new(link.from), new(link.to)) {
            transaction.execute(
                "INSERT INTO links (project_id, from_tile, to_tile) VALUES (?1, ?2, ?3)",
                params![project, from, to],
            )?;
        }
    }

    transaction.commit()?;

    Ok(numbers)
}

/// Brings the tiles and links of `project` back to `tiles` and `links`
/// (a version kept for "Undo", `test_projects/workspace.rs`), in one
/// transaction, **with the numbers they had**, so that the project in
/// memory and in the database stay the same. Every tile names a scheme
/// of the project.
pub fn restore_content(db: &UserDb, project: i64, tiles: &[Tile], links: &[Link]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "DELETE FROM tile_sources WHERE tile_id IN \
         (SELECT t.id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = ?1)",
        [project],
    )?;
    transaction.execute("DELETE FROM links WHERE project_id = ?1", [project])?;
    transaction.execute(
        "DELETE FROM tiles WHERE scheme_id IN (SELECT id FROM schemes WHERE project_id = ?1)",
        [project],
    )?;

    for tile in tiles {
        write_tile(&transaction, INSERT_TILE, tile.scheme, &tile.fields)?;

        // The tile gets back the number it had.
        let given = transaction.last_insert_rowid();
        transaction.execute("UPDATE tiles SET id = ?1 WHERE id = ?2", params![tile.id, given])?;
        write_sources(&transaction, tile.id, &tile.fields.sources)?;
    }

    for link in links {
        transaction.execute(
            "INSERT INTO links (id, project_id, from_tile, to_tile) VALUES (?1, ?2, ?3, ?4)",
            params![link.id, project, link.from, link.to],
        )?;
    }

    transaction.commit()?;

    Ok(())
}
