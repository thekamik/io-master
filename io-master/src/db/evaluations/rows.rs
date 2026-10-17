//! The rows of the evaluation projects and the Rust types they become
//! (`model/evaluations.rs`, `model/test_projects.rs`).

use rusqlite::Row;

use crate::model::evaluations::{
    Actor, Evaluation, EvaluationFields, JournalAction, JournalEntry, Origin, ResultField,
};
use crate::model::test_projects::{
    External, OutputSection, PageSpan, ProposalState, Scheme, SchemeFields, Tile, TileFields, TileKind,
};

/// The columns of an evaluation, in the order of [`evaluation_from_row`].
pub(super) const EVALUATION_COLUMNS: &str = "id, name, product, product_version, manufacturer, evaluator, \
source_name, source_version, source_author, source_description, imported";

pub(super) fn evaluation_from_row(row: &Row<'_>) -> rusqlite::Result<Evaluation> {
    Ok(Evaluation {
        id: row.get(0)?,
        fields: EvaluationFields {
            name: row.get(1)?,
            product: row.get(2)?,
            product_version: row.get(3)?,
            manufacturer: row.get(4)?,
            evaluator: row.get(5)?,
        },
        origin: Origin { name: row.get(6)?, version: row.get(7)?, author: row.get(8)?, description: row.get(9)? },
        imported: row.get(10)?,
    })
}

/// A scheme from a row of `id, evaluation_id, number, name,
/// document_id, first_page, last_page`.
pub(super) fn scheme_from_row(row: &Row<'_>) -> rusqlite::Result<Scheme> {
    let document: Option<i64> = row.get(4)?;
    let first: Option<u32> = row.get(5)?;
    let last: Option<u32> = row.get(6)?;

    Ok(Scheme {
        id: row.get(0)?,
        project: row.get(1)?,
        fields: SchemeFields {
            number: row.get(2)?,
            name: row.get(3)?,
            pages: match (document, first, last) {
                (Some(document_id), Some(first_page), Some(last_page)) => {
                    Some(PageSpan { document_id, first_page, last_page })
                }
                _ => None,
            },
            // Only accepted schemes are copied.
            state: ProposalState::Accepted,
        },
    })
}

/// The columns of a tile `t`, in the order of [`tile_from_row`].
pub(super) const TILE_COLUMNS: &str = "t.id, t.scheme_id, t.kind, t.identifier, t.title, t.description, t.place, \
t.external_document, t.external_place, t.sections, t.instructions, t.x, t.y";

pub(super) fn tile_from_row(row: &Row<'_>) -> rusqlite::Result<Tile> {
    let kind: String = row.get(2)?;
    let external_document: Option<String> = row.get(7)?;
    let external_place: Option<String> = row.get(8)?;
    let sections: String = row.get(9)?;

    Ok(Tile {
        id: row.get(0)?,
        scheme: row.get(1)?,
        fields: TileFields {
            // Only valid names are written; a database changed by hand
            // falls back to the safe side, as in db/test_projects/.
            kind: TileKind::from_name(&kind).unwrap_or(TileKind::Requirement),
            identifier: row.get(3)?,
            title: row.get(4)?,
            description: row.get(5)?,
            place: row.get(6)?,
            sources: Vec::new(),
            // Only accepted tiles are copied.
            state: ProposalState::Accepted,
            external: external_document
                .map(|document| External { document, place: external_place.unwrap_or_default() }),
            sections: sections.split(',').filter_map(OutputSection::from_name).collect(),
            instructions: row.get(10)?,
            x: row.get(11)?,
            y: row.get(12)?,
        },
    })
}

/// The name of an actor in the journal, and the model it names.
pub(super) fn actor_columns(actor: &Actor) -> (&'static str, Option<&str>) {
    match actor {
        Actor::Evaluator => ("evaluator", None),
        Actor::Model(model) => ("model", Some(model.as_str())),
    }
}

/// An entry of the journal from a row of `id, time, actor, model,
/// action, tile_id, field, detail`.
pub(super) fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<JournalEntry> {
    let actor: String = row.get(2)?;
    let model: Option<String> = row.get(3)?;
    let action: String = row.get(4)?;
    let field: Option<String> = row.get(6)?;

    Ok(JournalEntry {
        id: row.get(0)?,
        time: row.get(1)?,
        // Only "evaluator" and "model" are written; a row changed by
        // hand shows the model it names, or an unnamed one.
        actor: if actor == "evaluator" { Actor::Evaluator } else { Actor::Model(model.unwrap_or_default()) },
        // An unknown action (a row changed by hand) is shown as a
        // result written: the safe side, a step of the person.
        action: JournalAction::from_name(&action).unwrap_or(JournalAction::Written),
        tile: row.get(5)?,
        field: field.as_deref().and_then(ResultField::from_name),
        detail: row.get(7)?,
    })
}
