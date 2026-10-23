//! The commands of the object chosen.

use serde_json::Value;
use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::model::oscal::catalog::{Control, Group};

use super::super::oscal_object;
use super::view::ObjectView;

/// The group or control `id` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_get(state: State<'_, AppState>, uuid: String, id: String) -> Result<ObjectView, AppError> {
    let session = state.session()?;
    let catalog = db_oscal::read::catalog(&session.db(), &uuid)?;

    let Some(location) = catalog.locate(&id)? else {
        return Err(OscalError::NoSuchObject(id).into());
    };

    ObjectView::new(&catalog, &location, &id)
}

/// Changes the own fields of group `id` of catalog `uuid` (`group`: its
/// JSON of OSCAL, the id may be new).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_save_group(
    state: State<'_, AppState>,
    uuid: String,
    id: String,
    group: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let group: Group = oscal_object::read(group)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.update_group(&id, group)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Changes the own fields of control `id` of catalog `uuid`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_save_control(
    state: State<'_, AppState>,
    uuid: String,
    id: String,
    control: Value,
) -> Result<(), AppError> {
    let session = state.session()?;
    let control: Control = oscal_object::read(control)?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    catalog.update_control(&id, control)?;
    db_oscal::update::catalog(&db, catalog)?;

    Ok(())
}

/// Deletes the group or control `id` of catalog `uuid` with everything it
/// holds, and the quotes of everything deleted.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn object_form_delete(state: State<'_, AppState>, uuid: String, id: String) -> Result<(), AppError> {
    let session = state.session()?;
    let db = session.db();

    let mut catalog = db_oscal::read::catalog(&db, &uuid)?;
    let gone = catalog.remove(&id)?;

    db_oscal::update::catalog(&db, catalog)?;
    db_evaluation_support::delete::quotes_of(&db, &uuid, &gone)?;

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
