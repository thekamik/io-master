//! The command of the tree.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::oscal as db_oscal;

use super::view::CatalogTreeView;

/// Catalog `uuid` as a tree of rows.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn catalog_tree_get(state: State<'_, AppState>, uuid: String) -> Result<CatalogTreeView, AppError> {
    let session = state.session()?;
    let catalog = db_oscal::read::catalog(&session.db(), &uuid)?;

    CatalogTreeView::new(&catalog)
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
