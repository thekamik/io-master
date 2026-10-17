//! Reading evaluation projects, their frozen structure, the results and
//! the proposals of the AI. (The journal: `journal/read.rs`.)

use rusqlite::{OptionalExtension, named_params};

use crate::db::error::DbError;
use crate::db::test_projects::rows::tiles_with_sources;
use crate::db::user::database::UserDb;
use crate::model::evaluations::{Evaluation, Proposal, RequirementResult};
use crate::model::test_projects::{Link, Scheme, Tile};

use super::rows::{
    evaluation_from_row, evaluation_tile_from_row, link_from_row, proposal_from_row, result_from_row, scheme_from_row,
};

/* ---------------------------------------------------------------
 * Evaluations and their structure
 * ------------------------------------------------------------- */

/// Every evaluation, by name.
pub fn evaluations(db: &UserDb) -> Result<Vec<Evaluation>, DbError> {
    let mut statement = db.connection().prepare("SELECT * FROM evaluations ORDER BY name COLLATE NOCASE, id")?;

    Ok(statement.query_map([], evaluation_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// One evaluation, without its structure; `None` when there is no such.
pub fn evaluation(db: &UserDb, id: i64) -> Result<Option<Evaluation>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT * FROM evaluations WHERE id = :id", named_params! { ":id": id }, evaluation_from_row)
        .optional()?)
}

/// Whether an evaluation has `name` (letter case ignored).
pub fn name_taken(db: &UserDb, name: &str) -> Result<bool, DbError> {
    Ok(db.connection().query_row(
        "SELECT EXISTS (SELECT 1 FROM evaluations WHERE name = :name COLLATE NOCASE)",
        named_params! { ":name": name },
        |row| row.get(0),
    )?)
}

/// The documents of the standard an evaluation uses, in their order.
pub fn documents(db: &UserDb, id: i64) -> Result<Vec<i64>, DbError> {
    let mut statement = db
        .connection()
        .prepare("SELECT document_id FROM evaluation_documents WHERE evaluation_id = :id ORDER BY position")?;

    Ok(statement.query_map(named_params! { ":id": id }, |row| row.get(0))?.collect::<Result<Vec<_>, _>>()?)
}

/// The schemes of an evaluation, in the order of the test project.
pub fn schemes(db: &UserDb, id: i64) -> Result<Vec<Scheme>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT * FROM evaluation_schemes WHERE evaluation_id = :id ORDER BY position, id")?;

    Ok(statement.query_map(named_params! { ":id": id }, scheme_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// The links of an evaluation.
pub fn links(db: &UserDb, id: i64) -> Result<Vec<Link>, DbError> {
    let mut statement =
        db.connection().prepare("SELECT * FROM evaluation_links WHERE evaluation_id = :id ORDER BY id")?;

    Ok(statement.query_map(named_params! { ":id": id }, link_from_row)?.collect::<Result<Vec<_>, _>>()?)
}

/// The tiles of an evaluation, with their sources.
pub fn tiles(db: &UserDb, id: i64) -> Result<Vec<Tile>, DbError> {
    Ok(tiles_with_sources(
        db.connection(),
        "SELECT * FROM evaluation_tiles WHERE evaluation_id = :key ORDER BY id",
        "SELECT s.tile_id, s.document_id, s.page, s.quote, s.found FROM evaluation_tile_sources s \
         JOIN evaluation_tiles t ON t.id = s.tile_id WHERE t.evaluation_id = :key ORDER BY s.id",
        id,
        evaluation_tile_from_row,
    )?)
}

/// One tile of an evaluation with its sources, and the evaluation it
/// belongs to; `None` when there is no such.
pub fn tile(db: &UserDb, id: i64) -> Result<Option<(i64, Tile)>, DbError> {
    let evaluation: Option<i64> = db
        .connection()
        .query_row("SELECT evaluation_id FROM evaluation_tiles WHERE id = :id", named_params! { ":id": id }, |row| {
            row.get(0)
        })
        .optional()?;

    let Some(evaluation) = evaluation else {
        return Ok(None);
    };

    let tile = tiles_with_sources(
        db.connection(),
        "SELECT * FROM evaluation_tiles WHERE id = :key",
        "SELECT tile_id, document_id, page, quote, found FROM evaluation_tile_sources WHERE tile_id = :key ORDER BY id",
        id,
        evaluation_tile_from_row,
    )?
    .pop();

    Ok(tile.map(|tile| (evaluation, tile)))
}

/* ---------------------------------------------------------------
 * Results and proposals
 * ------------------------------------------------------------- */

/// The accepted results of every requirement of an evaluation.
pub fn results_of(db: &UserDb, evaluation: i64) -> Result<Vec<RequirementResult>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT r.* FROM evaluation_results r JOIN evaluation_tiles t ON t.id = r.tile_id \
         WHERE t.evaluation_id = :evaluation ORDER BY r.tile_id",
    )?;

    Ok(statement
        .query_map(named_params! { ":evaluation": evaluation }, result_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// The proposals of the AI waiting in an evaluation, the oldest first:
/// one query for all of them.
pub fn proposals_of(db: &UserDb, evaluation: i64) -> Result<Vec<Proposal>, DbError> {
    let mut statement = db.connection().prepare(
        "SELECT p.* FROM evaluation_proposals p JOIN evaluation_tiles t ON t.id = p.tile_id \
         WHERE t.evaluation_id = :evaluation ORDER BY p.id",
    )?;

    Ok(statement
        .query_map(named_params! { ":evaluation": evaluation }, proposal_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// The requirement a proposal belongs to; `None` when there is no such.
pub fn proposal_tile(db: &UserDb, id: i64) -> Result<Option<i64>, DbError> {
    Ok(db
        .connection()
        .query_row("SELECT tile_id FROM evaluation_proposals WHERE id = :id", named_params! { ":id": id }, |row| {
            row.get(0)
        })
        .optional()?)
}
