//! Reading the test projects, their schemes, tiles and links.

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::test_projects::{Link, Project, Scheme, Tile};

use super::rows::{link_from_row, project_from_row, project_tile_from_row, scheme_from_row, tiles_with_sources};

/* ---------------------------------------------------------------
 * Projects
 * ------------------------------------------------------------- */

/// The documents a project uses, in the order the user gave them.
fn project_documents(db: &UserDb, id: i64) -> Result<Vec<i64>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT document_id FROM project_documents WHERE project_id = :id ORDER BY position")?;

    Ok(statement.query_map(named_params! { ":id": id }, |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?)
}

/// A project read from its row, with its documents.
fn with_documents(db: &UserDb, mut project: Project) -> Result<Project, DbError> {
    project.fields.documents = project_documents(db, project.id)?;

    Ok(project)
}

/// Every project, by name.
pub fn projects(db: &UserDb) -> Result<Vec<Project>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM projects ORDER BY name COLLATE NOCASE, id")?;
    let projects = statement.query_map([], project_from_row)?.collect::<Result<Vec<_>, _>>()?;

    projects.into_iter().map(|project| with_documents(db, project)).collect()
}

/// One project, without its schemes; `None` when there is no such.
pub fn project_by_id(db: &UserDb, id: i64) -> Result<Option<Project>, DbError> {
    let project = db
        .connection()
        .query_row("SELECT * FROM projects WHERE id = :id", named_params! { ":id": id }, project_from_row)
        .optional()?;

    project.map(|project| with_documents(db, project)).transpose()
}

/// The number of the project named `name` (letter case aside), if any.
pub fn project_named(db: &UserDb, name: &str) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT id FROM projects WHERE name = :name COLLATE NOCASE",
            named_params! { ":name": name },
            |row| row.get(0),
        )
        .optional()?)
}

/* ---------------------------------------------------------------
 * Schemes
 * ------------------------------------------------------------- */

/// The schemes of a project, in the order of its list.
pub fn schemes(db: &UserDb, project: i64) -> Result<Vec<Scheme>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT * FROM schemes WHERE project_id = :project ORDER BY position, id")?;

    Ok(statement.query_map(named_params! { ":project": project }, scheme_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// One scheme; `None` when there is no such.
pub fn scheme(db: &UserDb, id: i64) -> Result<Option<Scheme>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT * FROM schemes WHERE id = :id", named_params! { ":id": id }, scheme_from_row)
        .optional()?)
}

/* ---------------------------------------------------------------
 * Tiles
 * ------------------------------------------------------------- */

/// The tiles of all schemes of a project, in the order they were made.
pub fn project_tiles(db: &UserDb, project: i64) -> Result<Vec<Tile>, DbError> {
    Ok(tiles_with_sources(
        db.connection(),
        "SELECT t.* FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE c.project_id = :key ORDER BY t.id",
        "SELECT s.tile_id, s.document_id, s.page, s.quote, s.found FROM tile_sources s \
         JOIN tiles t ON t.id = s.tile_id JOIN schemes c ON c.id = t.scheme_id \
         WHERE c.project_id = :key ORDER BY s.id",
        project,
        project_tile_from_row,
    )?)
}

/// One tile; `None` when there is no such.
pub fn tile(db: &UserDb, id: i64) -> Result<Option<Tile>, DbError> {
    Ok(tiles_with_sources(
        db.connection(),
        "SELECT * FROM tiles WHERE id = :key",
        "SELECT tile_id, document_id, page, quote, found FROM tile_sources WHERE tile_id = :key ORDER BY id",
        id,
        project_tile_from_row,
    )?
    .pop())
}

/// The project a tile belongs to (through its scheme).
pub fn project_of_tile(db: &UserDb, id: i64) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT c.project_id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE t.id = :id",
            named_params! { ":id": id },
            |row| row.get(0),
        )
        .optional()?)
}

/* ---------------------------------------------------------------
 * Links
 * ------------------------------------------------------------- */

/// The links of a project.
pub fn links(db: &UserDb, project: i64) -> Result<Vec<Link>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM links WHERE project_id = :project ORDER BY id")?;

    Ok(statement.query_map(named_params! { ":project": project }, link_from_row)?.collect::<Result<Vec<_>, _>>()?)
}
