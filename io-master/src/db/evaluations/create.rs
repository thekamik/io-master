//! Creating evaluation projects, proposals of the AI, and entries of
//! the journal. The rules are checked by `product_evaluation/` before.
//!
//! **The journal is only added to** ([`journal`]): no file of this
//! folder changes or removes an entry, and the database refuses it as
//! well (two triggers, `schema.rs`). An entry is written in the same
//! transaction as the change it records, so there is never a change
//! without its entry, nor an entry of a change that did not happen.

use std::collections::HashMap;

use rusqlite::{Connection, params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::evaluation::scheme::{Link, Scheme, Tile};
use crate::product_evaluation::model::{Actor, EvaluationFields, JournalAction, Origin, ResultField};

use super::rows::actor_columns;

/// One entry of the journal to write.
#[derive(Debug, Clone)]
pub struct Step<'a> {
    pub actor: &'a Actor,
    pub action: JournalAction,
    pub tile: Option<i64>,
    pub field: Option<ResultField>,
    pub detail: &'a str,
}

/// Adds an entry to the journal of `evaluation`. `connection` is the
/// open transaction of the change the entry records (a `Transaction`
/// dereferences to a `Connection`).
pub(super) fn journal(connection: &Connection, evaluation: i64, step: &Step<'_>) -> rusqlite::Result<()> {
    let (actor, model) = actor_columns(step.actor);

    connection.execute(
        "INSERT INTO evaluation_journal (evaluation_id, actor, model, action, tile_id, field, detail) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            evaluation,
            actor,
            model,
            step.action.name(),
            step.tile,
            step.field.map(ResultField::name),
            step.detail
        ],
    )?;

    Ok(())
}

/// The frozen structure of a new evaluation, with the numbers it had
/// in its source: tiles name their scheme, links their tiles, by those
/// numbers; sources and pages name loaded documents.
#[derive(Debug, Clone)]
pub struct Copy<'a> {
    pub documents: &'a [i64],
    pub schemes: &'a [Scheme],
    pub tiles: &'a [Tile],
    pub links: &'a [Link],
}

/// Writes a new evaluation with its frozen structure and the first
/// entry of its journal (`step`), in one transaction; returns its
/// number. On an error nothing of it is left.
pub fn evaluation(
    db: &UserDb,
    fields: &EvaluationFields,
    origin: &Origin,
    copy: &Copy<'_>,
    step: &Step<'_>,
) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO evaluations (name, product, product_version, manufacturer, evaluator, \
         source_name, source_version, source_author, source_description) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            fields.name.trim(),
            fields.product.trim(),
            fields.product_version.trim(),
            fields.manufacturer.trim(),
            fields.evaluator.trim(),
            origin.name,
            origin.version,
            origin.author,
            origin.description
        ],
    )?;

    let id = transaction.last_insert_rowid();
    write_copy(&transaction, id, copy)?;
    journal(&transaction, id, step)?;
    transaction.commit()?;

    Ok(id)
}

/// Writes the documents, schemes, tiles with their sources and links
/// of evaluation `id`, numbering them anew.
fn write_copy(connection: &Connection, id: i64, copy: &Copy<'_>) -> Result<(), DbError> {
    for (position, document) in copy.documents.iter().enumerate() {
        connection.execute(
            "INSERT OR IGNORE INTO evaluation_documents (evaluation_id, document_id, position) VALUES (?1, ?2, ?3)",
            params![id, document, position as i64],
        )?;
    }

    // The number of every scheme and tile in its source, and its new one.
    let mut schemes: HashMap<i64, i64> = HashMap::new();

    for (position, scheme) in copy.schemes.iter().enumerate() {
        let pages = scheme.fields.pages;

        connection.execute(
            "INSERT INTO evaluation_schemes (evaluation_id, number, name, document_id, first_page, last_page, \
             position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                scheme.fields.number.trim(),
                scheme.fields.name.trim(),
                pages.map(|pages| pages.document_id),
                pages.map(|pages| pages.first_page),
                pages.map(|pages| pages.last_page),
                position as i64
            ],
        )?;
        schemes.insert(scheme.id, connection.last_insert_rowid());
    }

    let mut tiles: HashMap<i64, i64> = HashMap::new();

    for tile in copy.tiles {
        let scheme = *schemes
            .get(&tile.scheme)
            .ok_or_else(|| DbError::Sql(format!("tile {} names a scheme that is not copied", tile.id)))?;
        let fields = &tile.fields;
        let sections: Vec<&str> = fields.sections.iter().map(|section| section.name()).collect();

        connection.execute(
            "INSERT INTO evaluation_tiles (evaluation_id, scheme_id, kind, identifier, title, description, place, \
             external_document, external_place, sections, instructions, x, y) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                scheme,
                fields.kind.name(),
                fields.identifier,
                fields.title,
                fields.description,
                fields.place,
                fields.external.as_ref().map(|external| external.document.as_str()),
                fields.external.as_ref().map(|external| external.place.as_str()),
                sections.join(","),
                fields.instructions,
                fields.x,
                fields.y
            ],
        )?;

        let new = connection.last_insert_rowid();
        tiles.insert(tile.id, new);

        for source in &fields.sources {
            connection.execute(
                "INSERT INTO evaluation_tile_sources (tile_id, document_id, page, quote, found) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![new, source.document_id, source.page, source.quote, source.found],
            )?;
        }
    }

    for link in copy.links {
        let (Some(from), Some(to)) = (tiles.get(&link.from), tiles.get(&link.to)) else {
            return Err(DbError::Sql(format!("link {} names a tile that is not copied", link.id)));
        };

        connection.execute(
            "INSERT INTO evaluation_links (evaluation_id, from_tile, to_tile) VALUES (?1, ?2, ?3)",
            params![id, from, to],
        )?;
    }

    Ok(())
}

/// Stores a text the AI proposes for `field` of requirement `tile` of
/// `evaluation`, with its entry of the journal, in one transaction;
/// returns the number of the proposal.
pub fn proposal(
    db: &UserDb,
    evaluation: i64,
    tile: i64,
    field: ResultField,
    text: &str,
    model: &str,
    step: &Step<'_>,
) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO evaluation_proposals (tile_id, field, text, model) VALUES (?1, ?2, ?3, ?4)",
        params![tile, field.name(), text, model],
    )?;

    let id = transaction.last_insert_rowid();
    journal(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(id)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "create_tests.rs"]
mod tests;
