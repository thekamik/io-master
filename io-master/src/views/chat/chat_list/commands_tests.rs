//! Unit tests of `chat_list/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::views::chat::conversation::stored;
use crate::views::test_support::{logged_in_window, logged_out_window};

#[test]
fn the_chats_are_listed_the_newest_first() {
    let window = logged_in_window();
    let first = stored::start(&window.session().db(), "first").unwrap();
    let second = stored::start(&window.session().db(), "second").unwrap();

    let chats = window.call("chat_list_get", json!({})).unwrap();

    assert_eq!(chats[0]["id"], second);
    assert_eq!(chats[0]["title"], "second");
    assert_eq!(chats[1]["id"], first);
}

#[test]
fn the_list_needs_a_login() {
    assert_eq!(logged_out_window().call("chat_list_get", json!({})).unwrap_err()["kind"], "not_logged_in");
}
