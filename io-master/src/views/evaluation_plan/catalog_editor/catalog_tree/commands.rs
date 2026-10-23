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

#[cfg(test)]
mod tests {
    //! Unit tests of `catalog_tree/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::evaluation_plan::catalog_editor::test_support::{add_control, add_group, with_catalog};

    #[test]
    fn the_tree_is_rows_with_their_level() {
        let (window, uuid, _) = with_catalog();
        add_group(&window, &uuid, "ADV");
        add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();

        assert_eq!(tree["title"], "CC Part 3");
        assert_eq!(tree["version"], "CC:2022");
        assert_eq!(
            tree["rows"],
            json!([
                { "kind": "group", "id": "ADV", "title": "Group ADV", "level": 0 },
                { "kind": "control", "id": "ADV_FSP.4", "title": "Control ADV_FSP.4", "level": 1 }
            ])
        );
    }

    #[test]
    fn a_document_that_is_not_a_catalog_is_refused() {
        let (window, _, _) = with_catalog();

        assert_eq!(window.call("catalog_tree_get", json!({ "uuid": "no-such" })).unwrap_err()["kind"], "invalid");
    }
}
