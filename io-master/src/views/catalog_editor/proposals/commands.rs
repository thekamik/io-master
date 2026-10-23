//! The commands of the proposals.

use serde_json::Value;
use tauri::State;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::standards as db_standards;

use super::accept;
use super::view::{self, ProposalView};

/// The proposals waiting for catalog `uuid`, the oldest first.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_get(state: State<'_, UiState>, uuid: String) -> Result<Vec<ProposalView>, UiError> {
    let session = state.session()?;
    let db = session.db();

    view::all(db_evaluation_support::read::proposals(&db, &uuid)?, &db_standards::read::documents(&db)?)
}

/// Accepts proposal `id`, as it is (`corrected` null) or as corrected.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_accept(state: State<'_, UiState>, id: i64, corrected: Option<Value>) -> Result<(), UiError> {
    let session = state.session()?;

    accept::accept(&session.db(), id, corrected)?;

    Ok(())
}

/// Rejects proposal `id`: it is removed, the catalog is not touched.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_reject(state: State<'_, UiState>, id: i64) -> Result<(), UiError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_support::delete::proposal(&session.db(), id)?,
        UiError::Invalid("the proposal does not exist".to_owned())
    );

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
