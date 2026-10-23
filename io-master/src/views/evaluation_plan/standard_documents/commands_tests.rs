//! Unit tests of `standard_documents/commands.rs`, through the IPC of
//! Tauri. The loading opens a dialog of the system, so it is tested in
//! `load_tests.rs`.

use serde_json::json;

use crate::test_support::standards::load;
use crate::views::test_support::{logged_in_window, logged_out_window};

#[test]
fn a_document_is_listed_renamed_and_deleted() {
    let window = logged_in_window();
    let id = load(&window.session().db(), b"%PDF a");

    assert_eq!(window.call("standard_documents_get", json!({})).unwrap()[0]["page_count"], 3);

    let form = json!({ "title": " Common Criteria ", "part": "Part 3", "version": "CC:2022" });
    let changed = window.call("standard_documents_update", json!({ "id": id, "document": form })).unwrap();
    assert_eq!(changed["title"], "Common Criteria");
    assert_eq!(changed["part"], "Part 3");

    window.call("standard_documents_delete", json!({ "id": id })).unwrap();
    assert_eq!(window.call("standard_documents_get", json!({})).unwrap(), json!([]));
    assert_eq!(window.call("standard_documents_delete", json!({ "id": id })).unwrap_err()["kind"], "invalid");
}

#[test]
fn a_title_is_needed() {
    let window = logged_in_window();
    let id = load(&window.session().db(), b"%PDF b");
    let form = json!({ "title": "  ", "part": "", "version": "" });

    let error = window.call("standard_documents_update", json!({ "id": id, "document": form })).unwrap_err();

    assert_eq!(error["kind"], "invalid");
}

#[test]
fn the_documents_need_a_login() {
    assert_eq!(logged_out_window().call("standard_documents_get", json!({})).unwrap_err()["kind"], "not_logged_in");
}
