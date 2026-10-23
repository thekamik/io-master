//! Unit tests of `conversation/commands.rs`, through the IPC of Tauri. A
//! message with its reply is tested in `turn_tests.rs`: its events go
//! through a channel, which the mock window does not carry.

use serde_json::json;

use crate::views::chat::conversation::stored;
use crate::views::test_support::{logged_in_window, logged_out_window};

#[test]
fn a_chat_is_opened_and_deleted() {
    let window = logged_in_window();
    let id = stored::start(&window.session().db(), "What is ADV_FSP?").unwrap();

    let chat = window.call("conversation_open", json!({ "chatId": id })).unwrap();
    assert_eq!(chat["title"], "What is ADV_FSP?");
    assert_eq!(
        chat["messages"][0],
        json!({
            "speaker": "evaluator", "who": "You", "text": "What is ADV_FSP?",
            "created": chat["messages"][0]["created"], "notice": null
        })
    );

    window.call("conversation_delete", json!({ "chatId": id })).unwrap();

    assert_eq!(window.call("conversation_open", json!({ "chatId": id })).unwrap_err()["kind"], "invalid");
    assert_eq!(window.call("conversation_delete", json!({ "chatId": id })).unwrap_err()["kind"], "invalid");
    assert_eq!(window.call("chat_list_get", json!({})).unwrap(), json!([]));
}

#[test]
fn stop_without_a_reply_does_nothing() {
    logged_in_window().call("conversation_stop", json!({})).unwrap();
}

#[test]
fn the_chats_need_a_login() {
    let window = logged_out_window();

    assert_eq!(window.call("conversation_open", json!({ "chatId": 1 })).unwrap_err()["kind"], "not_logged_in");
}
