//! Unit tests of `connection_list/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::ai::api::connections;
use crate::model::ai::Connection;
use crate::views::test_support::{logged_in_window, logged_out_window};

#[test]
fn the_connections_are_listed_by_name_with_the_active_one() {
    let window = logged_in_window();
    let named = |name: &str| Connection { name: name.to_owned(), model: "m".to_owned(), ..Connection::template() };
    let first = connections::create(&window.session().db(), &named("B")).unwrap().id;
    let second = connections::create(&window.session().db(), &named("A")).unwrap().id;

    let list = window.call("connection_list_get", json!({})).unwrap();

    assert_eq!(list["connections"], json!([{ "id": second, "name": "A" }, { "id": first, "name": "B" }]));
    assert_eq!(list["active"], first);
}

#[test]
fn the_list_needs_a_login() {
    assert_eq!(logged_out_window().call("connection_list_get", json!({})).unwrap_err()["kind"], "not_logged_in");
}
