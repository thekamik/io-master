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
