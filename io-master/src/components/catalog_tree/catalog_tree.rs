//! The entry of the component "tree of a catalog": the groups and controls
//! of a catalog of OSCAL as rows with their level, drawn by the page without
//! recursion (page: `ui/src/components/catalog_tree/`, its command in
//! `ui/src/lib/api/components/catalog_tree.ts`). Used by the editor of a
//! catalog of the view Converter and by the browser of the standards of the
//! view Evaluation plan (step 51.1 of docs/history.md). The object chosen is
//! presentation, kept by the page.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::oscal as db_oscal;

use super::view::CatalogTreeView;

// ── When the component opens ──────────────────────────────────────────

/// Catalog `uuid` as a tree of rows.
///
/// Called by the page: `CatalogTree.svelte`, when it opens and after every
/// change of the catalog (`catalogTree.catalogTreeGet(uuid)`).
/// Calls: `db_oscal::read::catalog`, `CatalogTreeView::new` (the rows of the
/// tree, without recursion in the page).
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
    //! Tests of the command of the tree, called as the page calls it: by
    //! name, through the IPC of Tauri. The rows of a catalog with groups and
    //! controls are tested in the editor of a catalog, which makes them.

    use serde_json::json;

    use crate::views::converter::catalog_editor::test_support::with_catalog;

    #[test]
    fn an_empty_catalog_has_no_rows_and_an_unknown_one_is_refused() {
        let (window, uuid, _) = with_catalog();

        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();

        assert_eq!(tree["title"], "CC Part 3");
        assert_eq!(tree["rows"], json!([]));
        assert_eq!(window.call("catalog_tree_get", json!({ "uuid": "no-such" })).unwrap_err()["kind"], "invalid");
    }
}
