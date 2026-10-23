//! The command of the search.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
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
    state: State<'_, AppState>,
    text: String,
    document: Option<i64>,
) -> Result<Vec<HitView>, AppError> {
    let session = state.session()?;

    ensure!(
        text.chars().count() <= MAX_SEARCH,
        AppError::Invalid(format!("search for at most {MAX_SEARCH} characters"))
    );

    let db = session.db();

    Ok(view::all(search::search(&db, &text, document)?, &db_standards::read::documents(&db)?))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `document_search/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::test_support::standards::load;
    use crate::views::test_support::logged_in_window;

    #[test]
    fn the_hits_name_their_document_and_page() {
        let window = logged_in_window();
        let id = load(&window.session().db(), b"%PDF c");

        let hits = window.call("document_search_run", json!({ "text": "adv_fsp.4", "document": null })).unwrap();

        assert_eq!(hits[0]["document_id"], id);
        assert_eq!(hits[0]["document"], "Common Criteria — Part 3");
        assert_eq!(hits[0]["page"], 2);
    }

    #[test]
    fn a_too_long_text_is_refused() {
        let window = logged_in_window();
        let text = "x".repeat(1001);

        let error = window.call("document_search_run", json!({ "text": text, "document": null })).unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }
}
