//! The command of a new catalog.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::standards as db_standards;

use super::super::oscal_documents::view::OscalDocumentView;
use super::made;
use super::view::{self, SourceView};

/// The documents of a standard a catalog can come from, by title and
/// part.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_sources(state: State<'_, AppState>) -> Result<Vec<SourceView>, AppError> {
    let session = state.session()?;

    Ok(view::all(db_standards::read::documents(&session.db())?))
}

/// Makes a new, empty catalog named `title`, of `version`, from the
/// documents of a standard `sources`.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn new_catalog_create(
    state: State<'_, AppState>,
    title: String,
    version: String,
    sources: Vec<i64>,
) -> Result<OscalDocumentView, AppError> {
    let session = state.session()?;

    Ok(OscalDocumentView::new(made::create(&session.db(), &title, &version, &sources)?))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `new_catalog/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::test_support::standards::load;
    use crate::views::test_support::logged_in_window;

    #[test]
    fn a_catalog_is_made_from_documents() {
        let window = logged_in_window();
        let source = load(&window.session().db(), b"%PDF catalog");

        assert_eq!(window.call("new_catalog_sources", json!({})).unwrap()[0]["id"], source);

        let made = window
            .call("new_catalog_create", json!({ "title": "CC Part 3", "version": "CC:2022", "sources": [source] }))
            .unwrap();

        assert_eq!(made["model"], "catalog");
        assert_eq!(made["title"], "CC Part 3");
        assert_eq!(window.call("oscal_documents_get", json!({})).unwrap()[0]["uuid"], made["uuid"]);
    }

    #[test]
    fn a_source_that_does_not_exist_is_refused() {
        let window = logged_in_window();

        let error =
            window.call("new_catalog_create", json!({ "title": "X", "version": "", "sources": [99] })).unwrap_err();

        assert_eq!(error["kind"], "invalid");
    }
}
