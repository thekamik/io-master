//! The schemes of a test project, their tiles and the links of the
//! project, kept in the encrypted database of the evaluator
//! (`db/user_db.rs`):
//!
//! ```text
//! schemes        a chapter of the standard: number, name, its pages
//! tiles          the tiles of a scheme (scheme.rs), with their place
//! tile_sources   the quoted passages of every tile
//! links          the links of a project; their two tiles may be in
//!                two schemes of it
//! ```
//!
//! The project itself is stored by `projects.rs`. Every change is
//! checked by the rules of `scheme.rs` before it is stored, and a
//! source is checked against the text of its page (`quote.rs`):
//! whether its quote was found is kept with it, so that a passage the
//! AI made up stays visible as such. Changes of several rows are made
//! in one transaction: a project is never left half changed.

use std::collections::HashMap;
use std::fmt;

use rusqlite::{OptionalExtension, Row, params};

use crate::db::db::DbError;
use crate::db::user_db::UserDb;

use super::documents;
use super::projects;
use super::quote::is_on_page;
use super::scheme::{
    External, Link, OutputSection, PageSpan, Project, ProposalState, Scheme, SchemeError, SchemeFields, Source, Tile,
    TileFields, TileKind, check_link,
};

/// Why a change of a project was not stored.
#[derive(Debug)]
pub enum StoreError {
    /// It breaks a rule of a project.
    Scheme(SchemeError),
    /// The database failed.
    Db(DbError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Scheme(error) => write!(formatter, "{error}"),
            StoreError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<SchemeError> for StoreError {
    fn from(error: SchemeError) -> StoreError {
        StoreError::Scheme(error)
    }
}

impl From<DbError> for StoreError {
    fn from(error: DbError) -> StoreError {
        StoreError::Db(error)
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> StoreError {
        StoreError::Db(error.into())
    }
}

/* ---------------------------------------------------------------
 * Schemes
 * ------------------------------------------------------------- */

/// The columns of a scheme, in the order of [`scheme_from_row`].
const SCHEME_COLUMNS: &str = "id, project_id, number, name, state, document_id, first_page, last_page";

fn scheme_from_row(row: &Row<'_>) -> rusqlite::Result<Scheme> {
    let state: String = row.get(4)?;
    let document: Option<i64> = row.get(5)?;
    let first: Option<u32> = row.get(6)?;
    let last: Option<u32> = row.get(7)?;

    Ok(Scheme {
        id: row.get(0)?,
        project: row.get(1)?,
        fields: SchemeFields {
            number: row.get(2)?,
            name: row.get(3)?,
            pages: match (document, first, last) {
                (Some(document_id), Some(first_page), Some(last_page)) => Some(PageSpan {
                    document_id,
                    first_page,
                    last_page,
                }),
                _ => None,
            },
            state: ProposalState::from_name(&state).unwrap_or(ProposalState::Proposed),
        },
    })
}

/// The schemes of a project, in the order of its list.
pub fn of_project(db: &UserDb, project: i64) -> Result<Vec<Scheme>, DbError> {
    let mut statement = db.connection().prepare(&format!(
        "SELECT {SCHEME_COLUMNS} FROM schemes WHERE project_id = ?1 ORDER BY position, id"
    ))?;
    let schemes = statement.query_map([project], scheme_from_row)?.collect::<Result<Vec<_>, _>>()?;

    Ok(schemes)
}

/// One scheme; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Scheme>, DbError> {
    let scheme = db
        .connection()
        .query_row(&format!("SELECT {SCHEME_COLUMNS} FROM schemes WHERE id = ?1"), [id], scheme_from_row)
        .optional()?;

    Ok(scheme)
}

/// The scheme, or the error that it does not exist.
pub fn existing(db: &UserDb, id: i64) -> Result<Scheme, StoreError> {
    get(db, id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("scheme")))
}

/// Checks the fields of a scheme of `project`: its pages in a document
/// of the project, its number not used by another scheme of it.
fn check_scheme(db: &UserDb, project: &Project, id: Option<i64>, fields: &SchemeFields) -> Result<(), StoreError> {
    fields.check()?;

    if let Some(pages) = fields.pages {
        if !project.fields.documents.contains(&pages.document_id) {
            return Err(SchemeError::Invalid(
                "pages",
                "the pages of a scheme must be in a document of its project".to_owned(),
            )
            .into());
        }

        let count = documents::get(db, pages.document_id)?.map_or(0, |document| document.page_count);

        if pages.last_page > count {
            return Err(SchemeError::Invalid("pages", format!("the document has only {count} pages")).into());
        }
    }

    let number = fields.number.trim();

    if !number.is_empty()
        && of_project(db, project.id)?
            .iter()
            .any(|other| Some(other.id) != id && other.fields.number.trim().eq_ignore_ascii_case(number))
    {
        return Err(SchemeError::Invalid("number", format!("another scheme of the project has the number {number}")).into());
    }

    Ok(())
}

/// The values of the columns of a scheme (?2 to ?7).
fn write_scheme(connection: &rusqlite::Connection, sql: &str, first: i64, fields: &SchemeFields) -> rusqlite::Result<usize> {
    connection.execute(
        sql,
        params![
            first,
            fields.number.trim(),
            fields.name.trim(),
            fields.state.name(),
            fields.pages.map(|pages| pages.document_id),
            fields.pages.map(|pages| pages.first_page),
            fields.pages.map(|pages| pages.last_page),
        ],
    )
}

/// Adds a scheme at the end of the list of `project`; returns its
/// number.
pub fn create(db: &UserDb, project: i64, fields: &SchemeFields) -> Result<i64, StoreError> {
    let project = projects::existing(db, project)?;
    check_scheme(db, &project, None, fields)?;

    write_scheme(
        db.connection(),
        "INSERT INTO schemes (project_id, number, name, state, document_id, first_page, last_page, position) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, \
         (SELECT COALESCE(MAX(position), 0) + 1 FROM schemes WHERE project_id = ?1))",
        project.id,
        fields,
    )?;

    Ok(db.connection().last_insert_rowid())
}

/// Changes the fields of a scheme.
pub fn update(db: &UserDb, id: i64, fields: &SchemeFields) -> Result<(), StoreError> {
    let scheme = existing(db, id)?;
    let project = projects::existing(db, scheme.project)?;
    check_scheme(db, &project, Some(id), fields)?;

    write_scheme(
        db.connection(),
        "UPDATE schemes SET number = ?2, name = ?3, state = ?4, document_id = ?5, first_page = ?6, last_page = ?7 \
         WHERE id = ?1",
        id,
        fields,
    )?;

    Ok(())
}

/// Accepts a scheme proposed by the AI.
pub fn accept(db: &UserDb, id: i64) -> Result<(), StoreError> {
    let changed = db.connection().execute(
        "UPDATE schemes SET state = ?1 WHERE id = ?2",
        params![ProposalState::Accepted.name(), id],
    )?;

    if changed == 0 {
        return Err(SchemeError::NotFound("scheme").into());
    }

    Ok(())
}

/// Deletes a scheme with its tiles and their links (also the links
/// to tiles of other schemes); returns whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
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

/* ---------------------------------------------------------------
 * Tiles
 * ------------------------------------------------------------- */

/// The columns of a tile `t`, in the order of [`tile_from_row`].
const TILE_COLUMNS: &str = "t.id, t.scheme_id, t.kind, t.identifier, t.title, t.description, t.place, t.state, \
t.external_document, t.external_place, t.sections, t.instructions, t.x, t.y";

fn tile_from_row(row: &Row<'_>) -> rusqlite::Result<Tile> {
    let kind: String = row.get(2)?;
    let state: String = row.get(7)?;
    let external_document: Option<String> = row.get(8)?;
    let external_place: Option<String> = row.get(9)?;
    let sections: String = row.get(10)?;

    Ok(Tile {
        id: row.get(0)?,
        scheme: row.get(1)?,
        fields: TileFields {
            // Only valid names are written; a database changed by hand
            // falls back to the safe side.
            kind: TileKind::from_name(&kind).unwrap_or(TileKind::Requirement),
            identifier: row.get(3)?,
            title: row.get(4)?,
            description: row.get(5)?,
            place: row.get(6)?,
            sources: Vec::new(),
            state: ProposalState::from_name(&state).unwrap_or(ProposalState::Proposed),
            external: external_document.map(|document| External {
                document,
                place: external_place.unwrap_or_default(),
            }),
            sections: sections.split(',').filter_map(OutputSection::from_name).collect(),
            instructions: row.get(11)?,
            x: row.get(12)?,
            y: row.get(13)?,
        },
    })
}

/// Reads the tiles `tiles_sql` gives (the columns of [`TILE_COLUMNS`],
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
    let project = db
        .connection()
        .query_row(
            "SELECT c.project_id FROM tiles t JOIN schemes c ON c.id = t.scheme_id WHERE t.id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;

    Ok(project)
}

/// Checks a tile for its project, and finds its quotes on their pages:
/// the sources returned carry `found`.
fn checked_sources(db: &UserDb, project: &Project, fields: &TileFields) -> Result<Vec<Source>, StoreError> {
    fields.check()?;

    let mut checked = Vec::with_capacity(fields.sources.len());

    for source in &fields.sources {
        if !project.fields.documents.contains(&source.document_id) {
            return Err(SchemeError::Invalid(
                "sources",
                "a source quotes a document the project does not use; add it to the project first".to_owned(),
            )
            .into());
        }

        let text = documents::page(db, source.document_id, source.page)?.ok_or_else(|| {
            SchemeError::Invalid("sources", format!("the document has no page {}", source.page))
        })?;

        checked.push(Source {
            found: is_on_page(&source.quote, &text),
            ..source.clone()
        });
    }

    Ok(checked)
}

/// The values of the columns of a tile (?2 to ?13).
fn write_tile(
    connection: &rusqlite::Connection,
    sql: &str,
    first: i64,
    fields: &TileFields,
) -> rusqlite::Result<usize> {
    let sections: Vec<&str> = fields.sections.iter().map(|section| section.name()).collect();

    connection.execute(
        sql,
        params![
            first,
            fields.kind.name(),
            fields.identifier.trim(),
            fields.title.trim(),
            fields.description,
            fields.place.trim(),
            fields.state.name(),
            fields.external.as_ref().map(|external| external.document.trim()),
            fields.external.as_ref().map(|external| external.place.trim()),
            sections.join(","),
            fields.instructions,
            fields.x,
            fields.y,
        ],
    )
}

/// The statement that adds a tile to the scheme `?1`.
const INSERT_TILE: &str = "INSERT INTO tiles (scheme_id, kind, identifier, title, description, place, state, \
external_document, external_place, sections, instructions, x, y) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, \
?11, ?12, ?13)";

/// Writes the sources of a tile, replacing the earlier ones.
fn write_sources(connection: &rusqlite::Connection, tile: i64, sources: &[Source]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM tile_sources WHERE tile_id = ?1", [tile])?;

    for source in sources {
        connection.execute(
            "INSERT INTO tile_sources (tile_id, document_id, page, quote, found) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![tile, source.document_id, source.page, source.quote, source.found],
        )?;
    }

    Ok(())
}

/// Adds a tile to a scheme; returns its number.
pub fn add_tile(db: &UserDb, scheme: i64, fields: &TileFields) -> Result<i64, StoreError> {
    let scheme = existing(db, scheme)?;
    let project = projects::existing(db, scheme.project)?;
    let sources = checked_sources(db, &project, fields)?;

    let transaction = db.connection().unchecked_transaction()?;

    write_tile(&transaction, INSERT_TILE, scheme.id, fields)?;

    let id = transaction.last_insert_rowid();
    write_sources(&transaction, id, &sources)?;
    transaction.commit()?;

    Ok(id)
}

/// Changes a tile. Its kind can be changed only while no link joins
/// it: a link is allowed by the kinds it joins.
pub fn update_tile(db: &UserDb, id: i64, fields: &TileFields) -> Result<(), StoreError> {
    let old = tile(db, id)?.ok_or(StoreError::Scheme(SchemeError::NotFound("tile")))?;
    let scheme = existing(db, old.scheme)?;
    let project = projects::existing(db, scheme.project)?;
    let sources = checked_sources(db, &project, fields)?;

    if old.fields.kind != fields.kind && links(db, project.id)?.iter().any(|link| link.from == id || link.to == id) {
        return Err(SchemeError::Invalid(
            "kind",
            "the kind of a linked tile cannot be changed; remove its links first".to_owned(),
        )
        .into());
    }

    let transaction = db.connection().unchecked_transaction()?;

    write_tile(
        &transaction,
        "UPDATE tiles SET kind = ?2, identifier = ?3, title = ?4, description = ?5, place = ?6, state = ?7, \
         external_document = ?8, external_place = ?9, sections = ?10, instructions = ?11, x = ?12, y = ?13 \
         WHERE id = ?1",
        id,
        fields,
    )?;
    write_sources(&transaction, id, &sources)?;
    transaction.commit()?;

    Ok(())
}

/// Moves a tile on the canvas.
pub fn move_tile(db: &UserDb, id: i64, x: f64, y: f64) -> Result<(), StoreError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(SchemeError::Invalid("position", "the position is not a number".to_owned()).into());
    }

    if db.connection().execute("UPDATE tiles SET x = ?1, y = ?2 WHERE id = ?3", params![x, y, id])? == 0 {
        return Err(SchemeError::NotFound("tile").into());
    }

    Ok(())
}

/// Moves tiles to another scheme of their project (step 26); their
/// links stay, they belong to the project. Nothing is moved when one
/// of them is in another project.
pub fn move_to_scheme(db: &UserDb, ids: &[i64], scheme: i64) -> Result<(), StoreError> {
    let target = existing(db, scheme)?;

    for id in ids {
        match project_of_tile(db, *id)? {
            Some(project) if project == target.project => {}
            Some(_) => {
                return Err(SchemeError::Invalid(
                    "scheme",
                    "a tile can only be moved to a scheme of its own project".to_owned(),
                )
                .into());
            }
            None => return Err(SchemeError::NotFound("tile").into()),
        }
    }

    let transaction = db.connection().unchecked_transaction()?;

    for id in ids {
        transaction.execute("UPDATE tiles SET scheme_id = ?1 WHERE id = ?2", params![target.id, id])?;
    }

    transaction.commit()?;

    Ok(())
}

/// Accepts a tile proposed by the AI.
pub fn accept_tile(db: &UserDb, id: i64) -> Result<(), StoreError> {
    let changed = db.connection().execute(
        "UPDATE tiles SET state = ?1 WHERE id = ?2",
        params![ProposalState::Accepted.name(), id],
    )?;

    if changed == 0 {
        return Err(SchemeError::NotFound("tile").into());
    }

    Ok(())
}

/// Deletes a tile with its sources and links; returns whether it
/// existed.
pub fn delete_tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute("DELETE FROM tile_sources WHERE tile_id = ?1", [id])?;
    transaction.execute("DELETE FROM links WHERE from_tile = ?1 OR to_tile = ?1", [id])?;
    let deleted = transaction.execute("DELETE FROM tiles WHERE id = ?1", [id])? > 0;
    transaction.commit()?;

    Ok(deleted)
}

/* ---------------------------------------------------------------
 * Links
 * ------------------------------------------------------------- */

/// The links of a project.
pub fn links(db: &UserDb, project: i64) -> Result<Vec<Link>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT id, from_tile, to_tile FROM links WHERE project_id = ?1 ORDER BY id")?;

    let links = statement
        .query_map([project], |row| {
            Ok(Link {
                id: row.get(0)?,
                from: row.get(1)?,
                to: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(links)
}

/// Links two tiles of one project, also of two of its schemes, by the
/// rules of `scheme.rs` (checked against all the links of the
/// project); returns the number of the link.
pub fn add_link(db: &UserDb, from: i64, to: i64) -> Result<i64, StoreError> {
    let (Some(project), Some(other)) = (project_of_tile(db, from)?, project_of_tile(db, to)?) else {
        return Err(SchemeError::NotFound("tile").into());
    };

    if project != other {
        return Err(SchemeError::Invalid("link", "only tiles of one project can be linked".to_owned()).into());
    }

    let kinds: Vec<(i64, TileKind)> =
        project_tiles(db, project)?.into_iter().map(|tile| (tile.id, tile.fields.kind)).collect();
    check_link(&kinds, &links(db, project)?, from, to)?;

    db.connection().execute(
        "INSERT INTO links (project_id, from_tile, to_tile) VALUES (?1, ?2, ?3)",
        params![project, from, to],
    )?;

    Ok(db.connection().last_insert_rowid())
}

/// Removes a link; returns whether it existed.
pub fn delete_link(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute("DELETE FROM links WHERE id = ?1", [id])? > 0)
}

/* ---------------------------------------------------------------
 * The whole content
 * ------------------------------------------------------------- */

/// Replaces the tiles and links of a project by `tiles` and `links`
/// (as an earlier `projects::data` gave them: for "Undo", and for an
/// imported project). Every tile names its scheme, which must be a
/// scheme of the project. The tiles get new numbers; the links follow
/// them. Their sources are stored as they come, with their `found`,
/// which the caller has checked. Returns the new number of every old
/// one.
pub fn replace_content(
    db: &UserDb,
    project: i64,
    tiles: &[Tile],
    links: &[Link],
) -> Result<Vec<(i64, i64)>, StoreError> {
    let stored = projects::existing(db, project)?;
    let schemes: Vec<i64> = of_project(db, project)?.iter().map(|scheme| scheme.id).collect();

    for tile in tiles {
        tile.fields.check()?;

        if !schemes.contains(&tile.scheme) {
            return Err(SchemeError::NotFound("scheme").into());
        }

        if tile.fields.sources.iter().any(|source| !stored.fields.documents.contains(&source.document_id)) {
            return Err(
                SchemeError::Invalid("sources", "a source quotes a document the project does not use".to_owned()).into(),
            );
        }
    }

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
    let kinds: Vec<(i64, TileKind)> = tiles.iter().filter_map(|tile| Some((new(tile.id)?, tile.fields.kind))).collect();
    let mut written: Vec<Link> = Vec::new();

    for link in links {
        let (Some(from), Some(to)) = (new(link.from), new(link.to)) else {
            return Err(SchemeError::NotFound("tile").into());
        };

        check_link(&kinds, &written, from, to)?;

        transaction.execute(
            "INSERT INTO links (project_id, from_tile, to_tile) VALUES (?1, ?2, ?3)",
            params![project, from, to],
        )?;

        written.push(Link {
            id: transaction.last_insert_rowid(),
            from,
            to,
        });
    }

    transaction.commit()?;

    Ok(numbers)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "schemes_tests.rs"]
mod tests;
