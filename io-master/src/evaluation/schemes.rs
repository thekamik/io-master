//! The schemes of a test project, their tiles and the links of the
//! project, kept in the encrypted database of the evaluator
//! (`db/user/`):
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
//! in one transaction: a project is never left half changed. The SQL
//! is in `db/test_projects/`.

use std::fmt;

use crate::db::error::DbError;
use crate::db::test_projects::{create as store, delete as remove, read, update as change};
use crate::db::user::database::UserDb;

use super::documents;
use super::projects;
use super::quote::is_on_page;
use super::scheme::{Link, Project, Scheme, SchemeError, SchemeFields, Source, Tile, TileFields, TileKind, check_link};

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

/* ---------------------------------------------------------------
 * Schemes
 * ------------------------------------------------------------- */

/// The schemes of a project, in the order of its list.
pub fn of_project(db: &UserDb, project: i64) -> Result<Vec<Scheme>, DbError> {
    read::schemes(db, project)
}

/// One scheme; `None` when there is no such.
pub fn get(db: &UserDb, id: i64) -> Result<Option<Scheme>, DbError> {
    read::scheme(db, id)
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

/// Adds a scheme at the end of the list of `project`; returns its
/// number.
pub fn create(db: &UserDb, project: i64, fields: &SchemeFields) -> Result<i64, StoreError> {
    let project = projects::existing(db, project)?;
    check_scheme(db, &project, None, fields)?;

    Ok(store::scheme(db, project.id, fields)?)
}

/// Changes the fields of a scheme.
pub fn update(db: &UserDb, id: i64, fields: &SchemeFields) -> Result<(), StoreError> {
    let scheme = existing(db, id)?;
    let project = projects::existing(db, scheme.project)?;
    check_scheme(db, &project, Some(id), fields)?;

    Ok(change::scheme(db, id, fields)?)
}

/// Accepts a scheme proposed by the AI.
pub fn accept(db: &UserDb, id: i64) -> Result<(), StoreError> {
    if !change::accept_scheme(db, id)? {
        return Err(SchemeError::NotFound("scheme").into());
    }

    Ok(())
}

/// Deletes a scheme with its tiles and their links (also the links
/// to tiles of other schemes); returns whether it existed.
pub fn delete(db: &UserDb, id: i64) -> Result<bool, DbError> {
    remove::scheme(db, id)
}

/* ---------------------------------------------------------------
 * Tiles
 * ------------------------------------------------------------- */

/// The tiles of all schemes of a project, in the order they were made.
pub fn project_tiles(db: &UserDb, project: i64) -> Result<Vec<Tile>, DbError> {
    read::project_tiles(db, project)
}

/// One tile; `None` when there is no such.
pub fn tile(db: &UserDb, id: i64) -> Result<Option<Tile>, DbError> {
    read::tile(db, id)
}

/// The project a tile belongs to (through its scheme).
pub fn project_of_tile(db: &UserDb, id: i64) -> Result<Option<i64>, DbError> {
    read::project_of_tile(db, id)
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

/// Adds a tile to a scheme; returns its number.
pub fn add_tile(db: &UserDb, scheme: i64, fields: &TileFields) -> Result<i64, StoreError> {
    let scheme = existing(db, scheme)?;
    let project = projects::existing(db, scheme.project)?;
    let sources = checked_sources(db, &project, fields)?;

    Ok(store::tile(db, scheme.id, fields, &sources)?)
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

    Ok(change::tile(db, id, fields, &sources)?)
}

/// Moves a tile on the canvas.
pub fn move_tile(db: &UserDb, id: i64, x: f64, y: f64) -> Result<(), StoreError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(SchemeError::Invalid("position", "the position is not a number".to_owned()).into());
    }

    if !change::tile_position(db, id, x, y)? {
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

    Ok(change::tiles_scheme(db, ids, target.id)?)
}

/// Accepts a tile proposed by the AI.
pub fn accept_tile(db: &UserDb, id: i64) -> Result<(), StoreError> {
    if !change::accept_tile(db, id)? {
        return Err(SchemeError::NotFound("tile").into());
    }

    Ok(())
}

/// Deletes a tile with its sources and links; returns whether it
/// existed.
pub fn delete_tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
    remove::tile(db, id)
}

/* ---------------------------------------------------------------
 * Links
 * ------------------------------------------------------------- */

/// The links of a project.
pub fn links(db: &UserDb, project: i64) -> Result<Vec<Link>, DbError> {
    read::links(db, project)
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

    Ok(store::link(db, project, from, to)?)
}

/// Removes a link; returns whether it existed.
pub fn delete_link(db: &UserDb, id: i64) -> Result<bool, DbError> {
    remove::link(db, id)
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

    // The rules of the links, checked on the numbers the tiles have
    // now: the new numbers follow them one to one, so the graph is the
    // same. Nothing is written when one link is refused.
    let kinds: Vec<(i64, TileKind)> = tiles.iter().map(|tile| (tile.id, tile.fields.kind)).collect();
    let mut checked: Vec<Link> = Vec::with_capacity(links.len());

    for link in links {
        if !kinds.iter().any(|(id, _)| *id == link.from) || !kinds.iter().any(|(id, _)| *id == link.to) {
            return Err(SchemeError::NotFound("tile").into());
        }

        check_link(&kinds, &checked, link.from, link.to)?;
        checked.push(*link);
    }

    Ok(change::replace_content(db, project, tiles, links)?)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "schemes_tests.rs"]
mod tests;
