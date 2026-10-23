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

    let error = window.call("new_catalog_create", json!({ "title": "X", "version": "", "sources": [99] })).unwrap_err();

    assert_eq!(error["kind"], "invalid");
}
