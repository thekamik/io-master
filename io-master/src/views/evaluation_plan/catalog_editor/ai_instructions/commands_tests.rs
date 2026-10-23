//! Unit tests of `ai_instructions/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::views::test_support::logged_in_window;

#[test]
fn the_instructions_are_changed_and_the_default_comes_back() {
    let window = logged_in_window();
    let default = crate::ai::tasks::catalog_outline::DEFAULT_INSTRUCTIONS;

    assert_eq!(window.call("ai_instructions_get", json!({ "which": "catalog_outline" })).unwrap(), default);

    let saved = window.call("ai_instructions_save", json!({ "which": "catalog_outline", "text": "Read it all." }));
    assert_eq!(saved.unwrap(), "Read it all.");

    let back = window.call("ai_instructions_save", json!({ "which": "catalog_outline", "text": "" })).unwrap();
    assert_eq!(back, default);

    assert_eq!(window.call("ai_instructions_get", json!({ "which": "chat" })).unwrap_err()["kind"], "invalid");
}
