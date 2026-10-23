//! Unit tests of `api_key/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::ai::api::connections;
use crate::model::ai::Connection;
use crate::views::test_support::logged_in_window;

const API_KEY: &str = "sk-secret-of-anna";

#[test]
fn the_key_is_stored_and_never_given_back() {
    let window = logged_in_window();
    let connection = Connection {
        name: "Service".to_owned(),
        model: "m".to_owned(),
        headers: "authorization: Bearer {{api_key}}".to_owned(),
        ..Connection::template()
    };
    let id = connections::create(&window.session().db(), &connection).unwrap().id;

    let before = window.call("api_key_get", json!({ "id": id })).unwrap();
    assert_eq!(before, json!({ "has_key": false, "key_unused": false, "header_uses_key": true }));

    let stored = window.call("api_key_store", json!({ "id": id, "apiKey": format!("  {API_KEY}  ") })).unwrap();
    assert_eq!(stored["has_key"], true);
    assert!(!stored.to_string().contains(API_KEY));

    let deleted = window.call("api_key_delete", json!({ "id": id })).unwrap();
    assert_eq!(deleted["has_key"], false);
}

#[test]
fn a_key_no_header_uses_is_reported() {
    let window = logged_in_window();
    let connection = Connection { name: "Local".to_owned(), model: "m".to_owned(), ..Connection::template() };
    let id = connections::create(&window.session().db(), &connection).unwrap().id;

    let stored = window.call("api_key_store", json!({ "id": id, "apiKey": API_KEY })).unwrap();

    assert_eq!(stored["key_unused"], true);
}

#[test]
fn a_key_of_no_connection_is_refused() {
    let window = logged_in_window();

    assert_eq!(window.call("api_key_store", json!({ "id": 9, "apiKey": API_KEY })).unwrap_err()["kind"], "invalid");
    assert_eq!(window.call("api_key_get", json!({ "id": 9 })).unwrap_err()["kind"], "invalid");
}
