//! Unit tests of `connection_line/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::ai::api::connections;
use crate::model::ai::Connection;
use crate::views::test_support::logged_in_window;

#[test]
fn without_a_connection_the_line_is_empty() {
    assert_eq!(logged_in_window().call("connection_line_get", json!({})).unwrap(), json!(null));
}

#[test]
fn the_line_names_the_active_connection_and_its_model() {
    let window = logged_in_window();
    let connection = Connection { name: "Local".to_owned(), model: "llama".to_owned(), ..Connection::template() };
    connections::create(&window.session().db(), &connection).unwrap();

    let line = window.call("connection_line_get", json!({})).unwrap();

    assert_eq!(line, json!({ "name": "Local", "writer": "llama" }));
}
