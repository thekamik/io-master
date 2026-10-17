//! Changing test projects, schemes and tiles. The rules of
//! `model/test_projects/rules.rs` are checked by the callers
//! (`business/test_projects/`) before. A change of a row that does not exist is
//! the error `DbError::NoRow`; the functions that return `bool` say
//! whether the row existed instead.

use std::collections::HashMap;

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::model::test_projects::{Link, ProjectFields, ProposalState, SchemeFields, Source, Tile, TileFields};

use super::create::{documents, insert_tile, sources};
use super::delete::content;
use super::rows::sections_text;

/// The error for a change of `table` that found no row `id`.
fn changed(count: usize, table: &'static str, id: i64) -> Result<(), DbError> {
    if count == 0 { Err(DbError::NoRow { table, id }) } else { Ok(()) }
}

/// Changes the fields and the documents of a project, in one
/// transaction.
pub fn project(db: &UserDb, id: i64, fields: &ProjectFields) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let count = transaction.execute(
        "UPDATE projects SET name = :name, version = :version, description = :description, author = :author \
         WHERE id = :id",
        named_params! {
            ":id": id,
            ":name": fields.name,
            ":version": fields.version,
            ":description": fields.description,
            ":author": fields.author,
        },
    )?;
    changed(count, "projects", id)?;
    documents(&transaction, id, &fields.documents)?;
    transaction.commit()?;

    Ok(())
}

/// Marks a project ready, or a draft again.
pub fn ready(db: &UserDb, id: i64, ready: bool) -> Result<(), DbError> {
    let count = db
        .connection()
        .execute("UPDATE projects SET ready = :ready WHERE id = :id", named_params! { ":ready": ready, ":id": id })?;

    changed(count, "projects", id)
}

/// Changes the fields of a scheme.
pub fn scheme(db: &UserDb, id: i64, fields: &SchemeFields) -> Result<(), DbError> {
    let count = db.connection().execute(
        "UPDATE schemes SET number = :number, name = :name, state = :state, document_id = :document, \
         first_page = :first_page, last_page = :last_page WHERE id = :id",
        named_params! {
            ":id": id,
            ":number": fields.number,
            ":name": fields.name,
            ":state": fields.state.name(),
            ":document": fields.pages.map(|pages| pages.document_id),
            ":first_page": fields.pages.map(|pages| pages.first_page),
            ":last_page": fields.pages.map(|pages| pages.last_page),
        },
    )?;

    changed(count, "schemes", id)
}

/// Marks a scheme proposed by the AI accepted; returns whether it
/// exists.
pub fn accept_scheme(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "UPDATE schemes SET state = :state WHERE id = :id",
        named_params! { ":state": ProposalState::Accepted.name(), ":id": id },
    )? > 0)
}

/// Changes a tile and replaces its sources (already checked on their
/// pages), in one transaction.
pub fn tile(db: &UserDb, id: i64, fields: &TileFields, new_sources: &[Source]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    let count = transaction.execute(
        "UPDATE tiles SET kind = :kind, identifier = :identifier, title = :title, description = :description, \
         place = :place, state = :state, external_document = :external_document, external_place = :external_place, \
         sections = :sections, instructions = :instructions, x = :x, y = :y WHERE id = :id",
        named_params! {
            ":id": id,
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
    changed(count, "tiles", id)?;
    sources(&transaction, id, new_sources)?;
    transaction.commit()?;

    Ok(())
}

/// Moves a tile on the canvas; returns whether it exists.
pub fn tile_position(db: &UserDb, id: i64, x: f64, y: f64) -> Result<bool, DbError> {
    Ok(db
        .connection()
        .execute("UPDATE tiles SET x = :x, y = :y WHERE id = :id", named_params! { ":x": x, ":y": y, ":id": id })?
        > 0)
}

/// Moves tiles to the scheme `scheme`, in one transaction.
pub fn tiles_scheme(db: &UserDb, ids: &[i64], scheme: i64) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    for id in ids {
        let count = transaction.execute(
            "UPDATE tiles SET scheme_id = :scheme WHERE id = :id",
            named_params! { ":scheme": scheme, ":id": id },
        )?;
        changed(count, "tiles", *id)?;
    }

    transaction.commit()?;

    Ok(())
}

/// Marks a tile proposed by the AI accepted; returns whether it exists.
pub fn accept_tile(db: &UserDb, id: i64) -> Result<bool, DbError> {
    Ok(db.connection().execute(
        "UPDATE tiles SET state = :state WHERE id = :id",
        named_params! { ":state": ProposalState::Accepted.name(), ":id": id },
    )? > 0)
}

/// Replaces the tiles and links of `project` by `tiles` and `links`,
/// in one transaction. Every tile names its scheme; its sources are
/// stored as they come. The tiles get new numbers and the links follow
/// them; a link that names a tile not in `tiles` is skipped (the
/// caller has refused such links before). Returns the new number of
/// every old one.
pub fn replace_content(db: &UserDb, project: i64, tiles: &[Tile], links: &[Link]) -> Result<Vec<(i64, i64)>, DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    content(&transaction, project)?;

    let mut numbers = Vec::with_capacity(tiles.len());

    for tile in tiles {
        let id = insert_tile(&transaction, None, tile.scheme, &tile.fields)?;
        sources(&transaction, id, &tile.fields.sources)?;
        numbers.push((tile.id, id));
    }

    let new: HashMap<i64, i64> = numbers.iter().copied().collect();

    for link in links {
        if let (Some(from), Some(to)) = (new.get(&link.from), new.get(&link.to)) {
            transaction.execute(
                "INSERT INTO links (project_id, from_tile, to_tile) VALUES (:project, :from, :to)",
                named_params! { ":project": project, ":from": from, ":to": to },
            )?;
        }
    }

    transaction.commit()?;

    Ok(numbers)
}

/// Brings the tiles and links of `project` back to `tiles` and `links`
/// (a version kept for "Undo", `business/test_projects/workspace.rs`), in one
/// transaction, **with the numbers they had**, so that the project in
/// memory and in the database stay the same. Every tile names a scheme
/// of the project.
pub fn restore_content(db: &UserDb, project: i64, tiles: &[Tile], links: &[Link]) -> Result<(), DbError> {
    let transaction = db.connection().unchecked_transaction()?;

    content(&transaction, project)?;

    for tile in tiles {
        insert_tile(&transaction, Some(tile.id), tile.scheme, &tile.fields)?;
        sources(&transaction, tile.id, &tile.fields.sources)?;
    }

    for link in links {
        transaction.execute(
            "INSERT INTO links (id, project_id, from_tile, to_tile) VALUES (:id, :project, :from, :to)",
            named_params! { ":id": link.id, ":project": project, ":from": link.from, ":to": link.to },
        )?;
    }

    transaction.commit()?;

    Ok(())
}
