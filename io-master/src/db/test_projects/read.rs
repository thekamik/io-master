//! Reading the test projects, their schemes, tiles and links.

use std::collections::HashMap;

use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::evaluation::scheme::{Link, Project, ProjectFields, Scheme, Source, Tile};

use super::rows::{
    PROJECT_COLUMNS, ProjectRow, SCHEME_COLUMNS, TILE_COLUMNS, link_from_row, project_from_row, scheme_from_row,
    tile_from_row,
};

/* ---------------------------------------------------------------
 * Projects
 * ------------------------------------------------------------- */

/// The documents a project uses, in the order the user gave them.
pub fn project_documents(db: &UserDb, id: i64) -> Result<Vec<i64>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT document_id FROM project_documents WHERE project_id = ?1 ORDER BY position")?;
    let ids = statement.query_map([id], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?;

    Ok(ids)
}

/// A project from its row and its documents.
fn project(db: &UserDb, row: ProjectRow) -> Result<Project, DbError> {
    let (id, name, version, description, author, ready) = row;

    Ok(Project {
        id,
        fields: ProjectFields {
            name,
            version,
            description,
            author,
            documents: project_documents(db, id)?,
        },
        ready,
    })
}

/// Every project, by name.
pub fn projects(db: &UserDb) -> Result<Vec<Project>, DbError> {
    let mut statement = db
        .connection()
        .prepare(&format!("SELECT {PROJECT_COLUMNS} FROM projects ORDER BY name COLLATE NOCASE, id"))?;
    let rows = statement.query_map([], project_from_row)?.collect::<Result<Vec<_>, _>>()?;

    rows.into_iter().map(|row| project(db, row)).collect()
}

/// One project, without its schemes; `None` when there is no such.
pub fn project_by_id(db: &UserDb, id: i64) -> Result<Option<Project>, DbError> {
    let row = db
        .connection()
        .query_row(&format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?1"), [id], project_from_row)
        .optional()?;

    row.map(|row| project(db, row)).transpose()
}

/// The number of the project named `name` (letter case aside), if any.
pub fn project_named(db: &UserDb, name: &str) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT id FROM projects WHERE name = ?1 COLLATE NOCASE", [name], |row| row.get(0))
        .optional()?)
}

/// The documents a project needs: quoted by one of its tiles, or
/// holding the pages of one of its schemes.
pub fn documents_in_use(db: &UserDb, id: i64) -> Result<Vec<i64>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT s.document_id FROM tile_sources s JOIN tiles t ON t.id = s.tile_id \
         JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = ?1 \
         UNION SELECT document_id FROM schemes WHERE project_id = ?1 AND document_id IS NOT NULL",
    )?;

    Ok(statement.query_map([id], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?)
}

/* ---------------------------------------------------------------
 * Schemes
 * ------------------------------------------------------------- */

/// The schemes of a project, in the order of its list.
pub fn schemes(db: &UserDb, project: i64) -> Result<Vec<Scheme>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT {SCHEME_COLUMNS} FROM schemes WHERE project_id = ?1 ORDER BY position, id"
    ))?;

    Ok(statement.query_map([project], scheme_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// One scheme; `None` when there is no such.
pub fn scheme(db: &UserDb, id: i64) -> Result<Option<Scheme>, DbError> {
    Ok(db
        .connection()
        .query_row(&format!("SELECT {SCHEME_COLUMNS} FROM schemes WHERE id = ?1"), [id], scheme_from_row)
        .optional()?)
}

/* ---------------------------------------------------------------
 * Tiles
 * ------------------------------------------------------------- */

/// Reads the tiles `tiles_sql` gives (the columns of `TILE_COLUMNS`,
/// with `?1` = `key`), and fills in their sources, read by
/// `sources_sql` (tile, document, page, quote, found; with `?1`) in
/// one query for all of them, not one per tile.
fn read_tiles(db: &UserDb, tiles_sql: &str, sources_sql: &str, key: i64) -> Result<Vec<Tile>, DbError> {
    let mut statement = db.connection().prepare(tiles_sql)?;
    let mut tiles = statement.query_map([key], tile_from_row)?.collect::<Result<Vec<_>, _>>()?;

    let mut statement = db.connection().prepare(sources_sql)?;
    let rows = statement.query_map([key], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            Source {
                document_id: row.get(1)?,
                page: row.get(2)?,
                quote: row.get(3)?,
                found: row.get(4)?,
            },
        ))
    })?;

    let mut sources: HashMap<i64, Vec<Source>> = HashMap::new();

    for row in rows {
        let (tile, source) = row?;
        sources.entry(tile).or_default().push(source);
    }

    for tile in &mut tiles {
        tile.fields.sources = sources.remove(&tile.id).unwrap_or_default();
    }

    Ok(tiles)
}

/// The tiles of all schemes of a project, in the order they were made.
pub fn project_tiles(db: &UserDb, project: i64) -> Result<Vec<Tile>, DbError> {
    read_tiles(
        db,
        &format!(
            "SELECT {TILE_COLUMNS} FROM tiles t JOIN schemes c ON c.id = t.scheme_id \
             WHERE c.project_id = ?1 ORDER BY t.id"
        ),
        "SELECT s.tile_id, s.document_id, s.page, s.quote, s.found FROM tile_sources s \
         JOIN tiles t ON t.id = s.tile_id JOIN schemes c ON c.id = t.scheme_id \
         WHERE c.project_id = ?1 ORDER BY s.id",
        project,
    )
}

/// One tile; `None` when there is no such.
pub fn tile(db: &UserDb, id: i64) -> Result<Option<Tile>, DbError> {
    Ok(read_tiles(
        db,
        &format!("SELECT {TILE_COLUMNS} FROM tiles t WHERE t.id = ?1"),
        "SELECT tile_id, document_id, page, quote, found FROM tile_sources WHERE tile_id = ?1 ORDER BY id",
        id,
    )?
    .pop())
}

/// The project a tile belongs to (through its scheme).
pub fn project_of_tile(db: &UserDb, id: i64) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT c.project_id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE t.id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?)
}

/* ---------------------------------------------------------------
 * Links
 * ------------------------------------------------------------- */

/// The links of a project.
pub fn links(db: &UserDb, project: i64) -> Result<Vec<Link>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT id, from_tile, to_tile FROM links WHERE project_id = ?1 ORDER BY id")?;

    Ok(statement.query_map([project], link_from_row)?.collect::<Result<Vec<_>, _>>()?)
}
