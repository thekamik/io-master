//! The command of the search.

use tauri::State;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::db::standards as db_standards;

use super::search;
use super::view::{self, HitView};

/// The longest text searched for, in characters.
pub const MAX_SEARCH: usize = 1000;

/// Searches the text of the documents (all, or `document`).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn document_search_run(
    state: State<'_, UiState>,
    text: String,
    document: Option<i64>,
) -> Result<Vec<HitView>, UiError> {
    let session = state.session()?;

    ensure!(
        text.chars().count() <= MAX_SEARCH,
        UiError::Invalid(format!("search for at most {MAX_SEARCH} characters"))
    );

    let db = session.db();

    Ok(view::all(search::search(&db, &text, document)?, &db_standards::read::documents(&db)?))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
