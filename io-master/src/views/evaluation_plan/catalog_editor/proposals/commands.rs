//! The commands of the proposals.

use serde_json::Value;
use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::standards as db_standards;

use super::accept;
use super::view::{self, ProposalView};

/// The proposals waiting for catalog `uuid`, the oldest first.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_get(state: State<'_, AppState>, uuid: String) -> Result<Vec<ProposalView>, AppError> {
    let session = state.session()?;
    let db = session.db();

    view::all(db_evaluation_support::read::proposals(&db, &uuid)?, &db_standards::read::documents(&db)?)
}

/// Accepts proposal `id`, as it is (`corrected` null) or as corrected.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_accept(state: State<'_, AppState>, id: i64, corrected: Option<Value>) -> Result<(), AppError> {
    let session = state.session()?;

    accept::accept(&session.db(), id, corrected)?;

    Ok(())
}

/// Rejects proposal `id`: it is removed, the catalog is not touched.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn proposals_reject(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state.session()?;

    ensure!(
        db_evaluation_support::delete::proposal(&session.db(), id)?,
        AppError::Invalid("the proposal does not exist".to_owned())
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `proposals/commands.rs`, through the IPC of Tauri. The
    //! proposals are made here as the AI would store them.

    use serde_json::json;

    use crate::db::evaluation_support as db_evaluation_support;
    use crate::db::evaluation_support::create::NewProposal;
    use crate::model::evaluation_support::{ProposalKind, QuoteDraft};
    use crate::views::evaluation_plan::catalog_editor::test_support::{add_group, with_catalog};
    use crate::views::test_support::TestWindow;

    /// Stores a proposal of control `id` for group ADV of catalog `uuid`.
    fn propose(window: &TestWindow, uuid: &str, source: i64, id: &str) -> i64 {
        let content = json!({ "id": id, "title": "Complete", "parts": [{ "name": "statement", "prose": "Shall." }] });
        let quotes = [QuoteDraft { source_document_id: source, page: 2, text: "provide".to_owned(), found: true }];
        let proposal = NewProposal {
            document_uuid: uuid,
            kind: ProposalKind::Control,
            parent_id: Some("ADV"),
            content: &content,
            quotes: &quotes,
            note: "From page 2.",
            model: "fake",
        };

        db_evaluation_support::create::proposal(&window.session().db(), &proposal).unwrap()
    }

    #[test]
    fn proposals_are_shown_accepted_corrected_and_rejected() {
        let (window, uuid, source) = with_catalog();
        add_group(&window, &uuid, "ADV");
        let first = propose(&window, &uuid, source, "ADV_FSP.4");
        let second = propose(&window, &uuid, source, "ADV_FSP.5");

        let waiting = window.call("proposals_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(waiting[0]["object_id"], "ADV_FSP.4");
        assert_eq!(waiting[0]["parts"][0]["prose"], "Shall.");
        assert_eq!(waiting[0]["quotes"][0]["document"], "Common Criteria — Part 3");

        let corrected = json!({ "id": "ADV_FSP.4", "title": "Complete (corrected)" });
        window.call("proposals_accept", json!({ "id": first, "corrected": corrected })).unwrap();
        window.call("proposals_reject", json!({ "id": second })).unwrap();

        assert_eq!(window.call("proposals_get", json!({ "uuid": uuid })).unwrap(), json!([]));
        let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
        assert_eq!(tree["rows"][1]["title"], "Complete (corrected)");
        assert_eq!(window.call("proposals_reject", json!({ "id": second })).unwrap_err()["kind"], "invalid");
    }
}
