//! The rows of the evaluation projects and the Rust types they become
//! (`model/evaluations.rs`, `model/test_projects.rs`). Columns are read
//! by their names; a value the program never writes is an error
//! (`DbError::Corrupt`). The tiles of the frozen copy are read as the
//! tiles of a test project (`db/test_projects/rows.rs`); the journal has
//! its own rows (`journal/rows.rs`).

use rusqlite::Row;

use crate::db::error::named;
use crate::db::test_projects::rows::{pages_from_row, tile_from_row};
use crate::model::evaluations::{
    Evaluation, EvaluationFields, Origin, Proposal, RequirementResult, ResultField, ResultOrigin,
};
use crate::model::test_projects::{Link, ProposalState, Scheme, SchemeFields, Tile};

/// An evaluation from a row of `evaluations`.
pub(super) fn evaluation_from_row(row: &Row<'_>) -> rusqlite::Result<Evaluation> {
    Ok(Evaluation {
        id: row.get("id")?,
        fields: EvaluationFields {
            name: row.get("name")?,
            product: row.get("product")?,
            product_version: row.get("product_version")?,
            manufacturer: row.get("manufacturer")?,
            evaluator: row.get("evaluator")?,
        },
        origin: Origin {
            name: row.get("source_name")?,
            version: row.get("source_version")?,
            author: row.get("source_author")?,
            description: row.get("source_description")?,
        },
        imported: row.get("imported")?,
    })
}

/// A scheme of the frozen copy (only accepted schemes are copied).
pub(super) fn scheme_from_row(row: &Row<'_>) -> rusqlite::Result<Scheme> {
    Ok(Scheme {
        id: row.get("id")?,
        project: row.get("evaluation_id")?,
        fields: SchemeFields {
            number: row.get("number")?,
            name: row.get("name")?,
            pages: pages_from_row(row)?,
            state: ProposalState::Accepted,
        },
    })
}

/// A tile of the frozen copy (only accepted tiles are copied).
pub(super) fn evaluation_tile_from_row(row: &Row<'_>) -> rusqlite::Result<Tile> {
    tile_from_row(row, "evaluation_tiles", ProposalState::Accepted)
}

/// A link of the frozen copy.
pub(super) fn link_from_row(row: &Row<'_>) -> rusqlite::Result<Link> {
    Ok(Link { id: row.get("id")?, from: row.get("from_tile")?, to: row.get("to_tile")? })
}

/// An accepted result from a row of `evaluation_results`.
pub(super) fn result_from_row(row: &Row<'_>) -> rusqlite::Result<RequirementResult> {
    let field: String = row.get("field")?;
    let origin: String = row.get("origin")?;

    Ok(RequirementResult {
        tile: row.get("tile_id")?,
        field: named("evaluation_results", "field", &field, ResultField::from_name)?,
        text: row.get("text")?,
        origin: named("evaluation_results", "origin", &origin, ResultOrigin::from_name)?,
    })
}

/// A proposal of the AI from a row of `evaluation_proposals`.
pub(super) fn proposal_from_row(row: &Row<'_>) -> rusqlite::Result<Proposal> {
    let field: String = row.get("field")?;

    Ok(Proposal {
        id: row.get("id")?,
        tile: row.get("tile_id")?,
        field: named("evaluation_proposals", "field", &field, ResultField::from_name)?,
        text: row.get("text")?,
        model: row.get("model")?,
        created: row.get("created")?,
    })
}
