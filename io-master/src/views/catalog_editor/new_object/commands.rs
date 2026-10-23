//! The commands of a new object.

use serde_json::Value;
use tauri::State;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::oscal as db_oscal;
use crate::model::oscal::catalog::{Control, Group};

use super::super::oscal_object;
use super::view::PlaceForm;

/// Adds `group` (its JSON of OSCAL) at `place` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_object_add_group(
    state: State<'_, UiState>,
    uuid: String,
    place: PlaceForm,
    group: Value,
) -> Result<(), UiError> {
    let session = state.session()?;
    let place = place.into_place()?;
    let group: Group = oscal_object::read(group)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.add_group(&place, group)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Adds `control` (its JSON of OSCAL) at `place` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_object_add_control(
    state: State<'_, UiState>,
    uuid: String,
    place: PlaceForm,
    control: Value,
) -> Result<(), UiError> {
    let session = state.session()?;
    let place = place.into_place()?;
    let control: Control = oscal_object::read(control)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.add_control(&place, control)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
