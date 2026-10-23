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
