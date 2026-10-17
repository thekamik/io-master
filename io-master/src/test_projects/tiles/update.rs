//! Changing tiles of an open test project: the fields, the place on the
//! canvas, the acceptance of a proposal, the scheme they are in.
//! Written through to the database, then made in memory.

use crate::db::test_projects::update as store;
use crate::db::user::database::UserDb;

use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{ProposalState, TileFields};
use crate::test_projects::error::StoreError;
use crate::test_projects::schemes::read::existing as existing_scheme;
use crate::test_projects::workspace::OpenProject;

use super::create::{checked_sources, stored};
use super::read::existing;

/// Changes tile `id`. Its kind can be changed only while no link joins
/// it: a link is allowed by the kinds it joins.
pub fn tile(db: &UserDb, open: &mut OpenProject, id: i64, fields: &TileFields) -> Result<(), StoreError> {
    let old_kind = existing(open, id)?.fields.kind;
    let sources = checked_sources(db, open, fields)?;

    if old_kind != fields.kind && open.data().links.iter().any(|link| link.from == id || link.to == id) {
        return Err(SchemeError::Invalid(
            "kind",
            "the kind of a linked tile cannot be changed; remove its links first".to_owned(),
        )
        .into());
    }

    store::tile(db, id, fields, &sources)?;

    if let Some(tile) = open.data_mut().tiles.iter_mut().find(|tile| tile.id == id) {
        tile.fields = stored(fields, sources);
    }

    Ok(())
}

/// Moves tile `id` on the canvas.
pub fn position(db: &UserDb, open: &mut OpenProject, id: i64, x: f64, y: f64) -> Result<(), StoreError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(SchemeError::Invalid("position", "the position is not a number".to_owned()).into());
    }

    existing(open, id)?;

    if !store::tile_position(db, id, x, y)? {
        return Err(SchemeError::NotFound("tile").into());
    }

    if let Some(tile) = open.data_mut().tiles.iter_mut().find(|tile| tile.id == id) {
        tile.fields.x = x;
        tile.fields.y = y;
    }

    Ok(())
}

/// Accepts tile `id`, proposed by the AI.
pub fn accept(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<(), StoreError> {
    existing(open, id)?;

    if !store::accept_tile(db, id)? {
        return Err(SchemeError::NotFound("tile").into());
    }

    if let Some(tile) = open.data_mut().tiles.iter_mut().find(|tile| tile.id == id) {
        tile.fields.state = ProposalState::Accepted;
    }

    Ok(())
}

/// Moves tiles to another scheme of the open project (step 26); their
/// links stay, they belong to the project. Nothing is moved when one of
/// them is not a tile of the project.
pub fn to_scheme(db: &UserDb, open: &mut OpenProject, ids: &[i64], scheme: i64) -> Result<(), StoreError> {
    existing_scheme(open, scheme)?;

    for id in ids {
        if super::read::get(open, *id).is_none() {
            return Err(SchemeError::Invalid(
                "scheme",
                "a tile can only be moved to a scheme of its own project".to_owned(),
            )
            .into());
        }
    }

    store::tiles_scheme(db, ids, scheme)?;

    for tile in open.data_mut().tiles.iter_mut().filter(|tile| ids.contains(&tile.id)) {
        tile.scheme = scheme;
    }

    Ok(())
}
