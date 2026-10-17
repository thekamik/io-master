//! Creating evaluation projects and proposals of the AI, each with its
//! entry of the journal (`journal/create.rs`) in one transaction. The
//! rules are checked by `business/evaluations/` before, on the normal form of the
//! fields; this file writes what it gets.

use std::collections::HashMap;

use rusqlite::{Connection, named_params};

use crate::db::error::DbError;
use crate::db::test_projects::rows::sections_text;
use crate::db::user::database::UserDb;
use crate::model::evaluations::{EvaluationFields, Origin, ResultField};
use crate::model::test_projects::{Link, Scheme, Tile};

use super::journal::create::{JournalStep, entry};

/// The frozen structure of a new evaluation, with the numbers it had
/// in its source: tiles name their scheme, links their tiles, by those
/// numbers; sources and pages name loaded documents.
#[derive(Debug, Clone)]
pub struct FrozenCopy<'a> {
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
    copy: &FrozenCopy<'_>,
    step: &JournalStep<'_>,
) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO evaluations (name, product, product_version, manufacturer, evaluator, \
         source_name, source_version, source_author, source_description) \
         VALUES (:name, :product, :product_version, :manufacturer, :evaluator, \
         :source_name, :source_version, :source_author, :source_description)",
        named_params! {
            ":name": fields.name,
            ":product": fields.product,
            ":product_version": fields.product_version,
            ":manufacturer": fields.manufacturer,
            ":evaluator": fields.evaluator,
            ":source_name": origin.name,
            ":source_version": origin.version,
            ":source_author": origin.author,
            ":source_description": origin.description,
        },
    )?;

    let id = transaction.last_insert_rowid();
    write_copy(&transaction, id, copy)?;
    entry(&transaction, id, step)?;
    transaction.commit()?;

    Ok(id)
}

/// Writes the documents, schemes, tiles with their sources and links
/// of evaluation `id`, numbering them anew.
fn write_copy(connection: &Connection, id: i64, copy: &FrozenCopy<'_>) -> Result<(), DbError> {
    for (position, document) in copy.documents.iter().enumerate() {
        connection.execute(
            "INSERT INTO evaluation_documents (evaluation_id, document_id, position) \
             VALUES (:evaluation, :document, :position)",
            named_params! { ":evaluation": id, ":document": document, ":position": position as i64 },
        )?;
    }

    // The number of every scheme and tile in its source, and its new one.
    let mut schemes: HashMap<i64, i64> = HashMap::new();

    for (position, scheme) in copy.schemes.iter().enumerate() {
        let pages = scheme.fields.pages;

        connection.execute(
            "INSERT INTO evaluation_schemes (evaluation_id, number, name, document_id, first_page, last_page, \
             position) VALUES (:evaluation, :number, :name, :document, :first_page, :last_page, :position)",
            named_params! {
                ":evaluation": id,
                ":number": scheme.fields.number,
                ":name": scheme.fields.name,
                ":document": pages.map(|pages| pages.document_id),
                ":first_page": pages.map(|pages| pages.first_page),
                ":last_page": pages.map(|pages| pages.last_page),
                ":position": position as i64,
            },
        )?;
        schemes.insert(scheme.id, connection.last_insert_rowid());
    }

    let mut tiles: HashMap<i64, i64> = HashMap::new();

    for tile in copy.tiles {
        let scheme = *schemes
            .get(&tile.scheme)
            .ok_or_else(|| DbError::Inconsistent(format!("tile {} names a scheme that is not copied", tile.id)))?;
        let fields = &tile.fields;

        connection.execute(
            "INSERT INTO evaluation_tiles (evaluation_id, scheme_id, kind, identifier, title, description, place, \
             external_document, external_place, sections, instructions, x, y) \
             VALUES (:evaluation, :scheme, :kind, :identifier, :title, :description, :place, \
             :external_document, :external_place, :sections, :instructions, :x, :y)",
            named_params! {
                ":evaluation": id,
                ":scheme": scheme,
                ":kind": fields.kind.name(),
                ":identifier": fields.identifier,
                ":title": fields.title,
                ":description": fields.description,
                ":place": fields.place,
                ":external_document": fields.external.as_ref().map(|external| external.document.as_str()),
                ":external_place": fields.external.as_ref().map(|external| external.place.as_str()),
                ":sections": sections_text(&fields.sections),
                ":instructions": fields.instructions,
                ":x": fields.x,
                ":y": fields.y,
            },
        )?;

        let new = connection.last_insert_rowid();
        tiles.insert(tile.id, new);

        for source in &fields.sources {
            connection.execute(
                "INSERT INTO evaluation_tile_sources (tile_id, document_id, page, quote, found) \
                 VALUES (:tile, :document, :page, :quote, :found)",
                named_params! {
                    ":tile": new,
                    ":document": source.document_id,
                    ":page": source.page,
                    ":quote": source.quote,
                    ":found": source.found,
                },
            )?;
        }
    }

    for link in copy.links {
        let (Some(from), Some(to)) = (tiles.get(&link.from), tiles.get(&link.to)) else {
            return Err(DbError::Inconsistent(format!("link {} names a tile that is not copied", link.id)));
        };

        connection.execute(
            "INSERT INTO evaluation_links (evaluation_id, from_tile, to_tile) VALUES (:evaluation, :from, :to)",
            named_params! { ":evaluation": id, ":from": from, ":to": to },
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
    step: &JournalStep<'_>,
) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO evaluation_proposals (tile_id, field, text, model) VALUES (:tile, :field, :text, :model)",
        named_params! { ":tile": tile, ":field": field.name(), ":text": text, ":model": model },
    )?;

    let id = transaction.last_insert_rowid();
    entry(&transaction, evaluation, step)?;
    transaction.commit()?;

    Ok(id)
}
