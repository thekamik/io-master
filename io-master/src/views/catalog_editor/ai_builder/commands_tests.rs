//! Unit tests of `ai_builder/commands.rs`, through the IPC of Tauri. The
//! works themselves send their events through a channel, which the mock
//! window does not carry; they are tested in `groups_tests.rs` and
//! `controls_tests.rs`.

use serde_json::json;

use crate::views::catalog_editor::test_support::with_catalog;

#[test]
fn the_sources_are_the_documents_and_stop_without_a_work_does_nothing() {
    let (window, _, source) = with_catalog();

    let sources = window.call("ai_builder_sources", json!({})).unwrap();

    assert_eq!(sources, json!([{ "id": source, "title": "Common Criteria — Part 3", "page_count": 3 }]));
    window.call("ai_builder_stop", json!({})).unwrap();
}
