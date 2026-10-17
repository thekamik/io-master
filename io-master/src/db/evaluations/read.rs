//! Reading evaluation projects, their frozen structure, the results,
//! the proposals of the AI and the journal.

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::evaluation::model::{Evaluation, JournalEntry, Proposal, ResultField};
use crate::test_projects::model::{Link, Scheme, Source, Tile};

use super::rows::{
    EVALUATION_COLUMNS, TILE_COLUMNS, entry_from_row, evaluation_from_row, scheme_from_row, tile_from_row,
};

/* ---------------------------------------------------------------
 * Evaluations and their structure
 * ------------------------------------------------------------- */

/// Every evaluation, by name.
pub fn evaluations(db: &UserDb) -> Result<Vec<Evaluation>, DbError> {
    let mut statement = db
        .connection()
        .prepare(&format!("SELECT {EVALUATION_COLUMNS} FROM evaluations ORDER BY name COLLATE NOCASE, id"))?;

    Ok(statement.query_map([], evaluation_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// One evaluation, without its structure; `None` when there is no such.
pub fn evaluation(db: &UserDb, id: i64) -> Result<Option<Evaluation>, DbError> {
    Ok(db
        .connection()
        .query_row(&format!("SELECT {EVALUATION_COLUMNS} FROM evaluations WHERE id = ?1"), [id], evaluation_from_row)
        .optional()?)
}

/// Whether an evaluation has `name` (letter case ignored).
pub fn name_taken(db: &UserDb, name: &str) -> Result<bool, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT id FROM evaluations WHERE name = ?1 COLLATE NOCASE", [name.trim()], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .is_some())
}

/// The documents of the standard an evaluation uses, in their order.
pub fn documents(db: &UserDb, id: i64) -> Result<Vec<i64>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT document_id FROM evaluation_documents WHERE evaluation_id = ?1 ORDER BY position")?;

    Ok(statement.query_map([id], |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?)
}

/// The schemes of an evaluation, in the order of the test project.
pub fn schemes(db: &UserDb, id: i64) -> Result<Vec<Scheme>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT id, evaluation_id, number, name, document_id, first_page, last_page FROM evaluation_schemes \
         WHERE evaluation_id = ?1 ORDER BY position, id",
    )?;

    Ok(statement.query_map([id], scheme_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// The links of an evaluation.
pub fn links(db: &UserDb, id: i64) -> Result<Vec<Link>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT id, evaluation_id, from_tile, to_tile FROM evaluation_links WHERE evaluation_id = ?1 ORDER BY id",
    )?;
    let links = statement
        .query_map([id], |row| Ok(Link { id: row.get(0)?, from: row.get(2)?, to: row.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(links)
}

/// The tiles of an evaluation, with their sources.
pub fn tiles(db: &UserDb, id: i64) -> Result<Vec<Tile>, DbError> {
    tiles_where(db, "t.evaluation_id = ?1", id)
}

/// One tile of an evaluation with its sources, and the evaluation it
/// belongs to; `None` when there is no such.
pub fn tile(db: &UserDb, id: i64) -> Result<Option<(i64, Tile)>, DbError> {
    let evaluation: Option<i64> = db
        .connection()
        .query_row("SELECT evaluation_id FROM evaluation_tiles WHERE id = ?1", [id], |row| row.get(0))
        .optional()?;

    let Some(evaluation) = evaluation else {
        return Ok(None);
    };

    Ok(tiles_where(db, "t.id = ?1", id)?.pop().map(|tile| (evaluation, tile)))
}

/// The tiles that `condition` (on the table `t`, with `?1` = `key`)
/// selects, with their sources, read in one query for all of them.
fn tiles_where(db: &UserDb, condition: &str, key: i64) -> Result<Vec<Tile>, DbError> {
    let mut statement = db
        .connection()
        .prepare(&format!("SELECT {TILE_COLUMNS} FROM evaluation_tiles t WHERE {condition} ORDER BY t.id"))?;
    let mut tiles = statement.query_map([key], tile_from_row)?.collect::<Result<Vec<_>, _>>()?;

    let mut statement = db.connection().prepare(&format!(
        "SELECT s.tile_id, s.document_id, s.page, s.quote, s.found FROM evaluation_tile_sources s \
         JOIN evaluation_tiles t ON t.id = s.tile_id WHERE {condition} ORDER BY s.id"
    ))?;
    let rows = statement.query_map([key], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            Source { document_id: row.get(1)?, page: row.get(2)?, quote: row.get(3)?, found: row.get(4)? },
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

/* ---------------------------------------------------------------
 * Results and proposals
 * ------------------------------------------------------------- */

/// The accepted results of a requirement as stored: field, text,
/// origin (by their names, as `evaluation/results/` reads
/// them).
pub fn results(db: &UserDb, tile: i64) -> Result<Vec<(String, String, String)>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT field, text, origin FROM evaluation_results WHERE tile_id = ?1")?;

    Ok(statement
        .query_map([tile], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?)
}

/// The accepted text of a field, if any.
pub fn result_text(db: &UserDb, tile: i64, field: ResultField) -> Result<Option<String>, DbError> {
    Ok(db
        .connection()
        .query_row(
            "SELECT text FROM evaluation_results WHERE tile_id = ?1 AND field = ?2",
            params![tile, field.name()],
            |row| row.get(0),
        )
        .optional()?)
}

/// The proposals of the AI waiting for a requirement, the oldest
/// first; a row with an unknown field (changed by hand) is left out.
pub fn proposals(db: &UserDb, tile: i64) -> Result<Vec<Proposal>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT id, tile_id, field, text, model, created FROM evaluation_proposals WHERE tile_id = ?1 ORDER BY id",
    )?;
    let rows = statement
        .query_map([tile], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, tile, field, text, model, created)| {
            Some(Proposal { id, tile, field: ResultField::from_name(&field)?, text, model, created })
        })
        .collect())
}

/// The accepted results of every requirement of an evaluation as
/// stored: tile, field, text, origin (by their names).
pub fn results_of(db: &UserDb, evaluation: i64) -> Result<Vec<(i64, String, String, String)>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT r.tile_id, r.field, r.text, r.origin FROM evaluation_results r \
         JOIN evaluation_tiles t ON t.id = r.tile_id WHERE t.evaluation_id = ?1 ORDER BY r.tile_id",
    )?;

    Ok(statement
        .query_map([evaluation], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
        .collect::<Result<Vec<_>, _>>()?)
}

/// The proposals of the AI waiting in an evaluation, the oldest first.
pub fn proposals_of(db: &UserDb, evaluation: i64) -> Result<Vec<Proposal>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT p.tile_id FROM evaluation_proposals p JOIN evaluation_tiles t ON t.id = p.tile_id \
         WHERE t.evaluation_id = ?1 GROUP BY p.tile_id",
    )?;
    let tiles = statement.query_map([evaluation], |row| row.get::<_, i64>(0))?.collect::<Result<Vec<_>, _>>()?;
    let mut all = Vec::new();

    for tile in tiles {
        all.extend(proposals(db, tile)?);
    }

    all.sort_by_key(|proposal| proposal.id);

    Ok(all)
}

/// The requirement a proposal belongs to; `None` when there is no such.
pub fn proposal_tile(db: &UserDb, id: i64) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT tile_id FROM evaluation_proposals WHERE id = ?1", [id], |row| row.get(0))
        .optional()?)
}

/* ---------------------------------------------------------------
 * Journal
 * ------------------------------------------------------------- */

/// The journal of an evaluation, the oldest entry first.
pub fn journal(db: &UserDb, evaluation: i64) -> Result<Vec<JournalEntry>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT id, time, actor, model, action, tile_id, field, detail FROM evaluation_journal \
         WHERE evaluation_id = ?1 ORDER BY id",
    )?;

    Ok(statement.query_map([evaluation], entry_from_row)?.collect::<Result<Vec<_>, _>>()?)
}
