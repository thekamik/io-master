//! The command of a new catalog.

use tauri::State;

use crate::db::standards as db_standards;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

use super::super::oscal_documents::view::OscalDocumentView;
use super::made;
use super::view::{self, SourceView};

/// The documents of a standard a catalog can come from, by title and
/// part.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_sources(state: State<'_, UiState>) -> Result<Vec<SourceView>, UiError> {
    let session = state.session()?;

    Ok(view::all(db_standards::read::documents(&session.db())?))
}

/// Makes a new, empty catalog named `title`, of `version`, from the
/// documents of a standard `sources`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_create(
    state: State<'_, UiState>,
    title: String,
    version: String,
    sources: Vec<i64>,
) -> Result<OscalDocumentView, UiError> {
    let session = state.session()?;

    Ok(OscalDocumentView::new(made::create(&session.db(), &title, &version, &sources)?))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
