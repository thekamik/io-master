//! Creating test projects, schemes, tiles and links. The rules of
//! `model/test_projects/rules.rs` are checked by the callers
//! (`business/test_projects/`) before, on the normal form of the fields; this
//! file writes what it gets.

use rusqlite::{Connection, named_params};

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::test_projects::{ProjectFields, SchemeFields, Source, TileFields};

use super::rows::sections_text;

/// Stores a new project (not ready) with its documents, in one
/// transaction; returns its number.
pub fn project(db: &UserDb, fields: &ProjectFields) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    transaction.execute(
        "INSERT INTO projects (name, version, description, author, ready) \
         VALUES (:name, :version, :description, :author, 0)",
        named_params! {
            ":name": fields.name,
            ":version": fields.version,
            ":description": fields.description,
            ":author": fields.author,
        },
    )?;

    let id = transaction.last_insert_rowid();
    documents(&transaction, id, &fields.documents)?;
    transaction.commit()?;

    Ok(id)
}

/// Writes the documents of project `id`, replacing the earlier list,
/// in their order (also used by `update.rs`).
pub(super) fn documents(connection: &Connection, id: i64, documents: &[i64]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM project_documents WHERE project_id = :id", named_params! { ":id": id })?;

    for (position, document) in documents.iter().enumerate() {
        connection.execute(
            "INSERT INTO project_documents (project_id, document_id, position) VALUES (:project, :document, :position)",
            named_params! { ":project": id, ":document": document, ":position": position as i64 },
        )?;
    }

    Ok(())
}

/// Stores a scheme at the end of the list of `project`; returns its
/// number.
pub fn scheme(db: &UserDb, project: i64, fields: &SchemeFields) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO schemes (project_id, number, name, state, document_id, first_page, last_page, position) \
         VALUES (:project, :number, :name, :state, :document, :first_page, :last_page, \
         (SELECT COALESCE(MAX(position), 0) + 1 FROM schemes WHERE project_id = :project))",
        named_params! {
            ":project": project,
            ":number": fields.number,
            ":name": fields.name,
            ":state": fields.state.name(),
            ":document": fields.pages.map(|pages| pages.document_id),
            ":first_page": fields.pages.map(|pages| pages.first_page),
            ":last_page": fields.pages.map(|pages| pages.last_page),
        },
    )?;

    Ok(db.connection().last_insert_rowid())
}

/// Stores a tile of `scheme` with its `sources` (already checked on
/// their pages), in one transaction; returns its number.
pub fn tile(db: &UserDb, scheme: i64, fields: &TileFields, sources: &[Source]) -> Result<i64, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let id = insert_tile(&transaction, None, scheme, fields)?;
    self::sources(&transaction, id, sources)?;
    transaction.commit()?;

    Ok(id)
}

/// Inserts the row of a tile of `scheme`, with the number `id` when it
/// is given (a version brought back by "Undo", `update.rs`), else with
/// a new one; returns its number. Its sources are written apart.
pub(super) fn insert_tile(
    connection: &Connection,
    id: Option<i64>,
    scheme: i64,
    fields: &TileFields,
) -> rusqlite::Result<i64> {
    connection.execute(
        "INSERT INTO tiles (id, scheme_id, kind, identifier, title, description, place, state, \
         external_document, external_place, sections, instructions, x, y) \
         VALUES (:id, :scheme, :kind, :identifier, :title, :description, :place, :state, \
         :external_document, :external_place, :sections, :instructions, :x, :y)",
        named_params! {
            ":id": id,
            ":scheme": scheme,
            ":kind": fields.kind.name(),
            ":identifier": fields.identifier,
            ":title": fields.title,
            ":description": fields.description,
            ":place": fields.place,
            ":state": fields.state.name(),
            ":external_document": fields.external.as_ref().map(|external| external.document.as_str()),
            ":external_place": fields.external.as_ref().map(|external| external.place.as_str()),
            ":sections": sections_text(&fields.sections),
            ":instructions": fields.instructions,
            ":x": fields.x,
            ":y": fields.y,
        },
    )?;

    Ok(connection.last_insert_rowid())
}

/// Writes the sources of tile `tile`, replacing the earlier ones (also
/// used by `update.rs`).
pub(super) fn sources(connection: &Connection, tile: i64, sources: &[Source]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM tile_sources WHERE tile_id = :tile", named_params! { ":tile": tile })?;

    for source in sources {
        connection.execute(
            "INSERT INTO tile_sources (tile_id, document_id, page, quote, found) \
             VALUES (:tile, :document, :page, :quote, :found)",
            named_params! {
                ":tile": tile,
                ":document": source.document_id,
                ":page": source.page,
                ":quote": source.quote,
                ":found": source.found,
            },
        )?;
    }

    Ok(())
}

/// Stores a link of `project`; returns its number.
pub fn link(db: &UserDb, project: i64, from: i64, to: i64) -> Result<i64, DbError> {
    db.connection().execute(
        "INSERT INTO links (project_id, from_tile, to_tile) VALUES (:project, :from, :to)",
        named_params! { ":project": project, ":from": from, ":to": to },
    )?;

    Ok(db.connection().last_insert_rowid())
}
