//! Unit tests of `pdf_viewer/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::test_support::standards::load;
use crate::views::test_support::logged_in_window;

#[test]
fn the_pdf_of_a_document_is_given_and_of_no_document_refused() {
    let window = logged_in_window();
    let id = load(&window.session().db(), b"%PDF-1.7 viewer");

    assert_eq!(window.call("pdf_viewer_documents", json!({})).unwrap()[0]["page_count"], 3);
    assert_eq!(window.call_bytes("pdf_viewer_get", json!({ "id": id })).unwrap(), b"%PDF-1.7 viewer");
    assert_eq!(window.call_bytes("pdf_viewer_get", json!({ "id": id + 1 })).unwrap_err()["kind"], "invalid");
}
