//! Unit tests of `connection_form/commands.rs`, through the IPC of Tauri.

use serde_json::{Value, json};

use crate::model::ai::Connection;
use crate::views::settings::connection_form::view::ConnectionView;
use crate::views::test_support::{TestWindow, logged_in_window, logged_out_window};

/// The form the page sends for `connection`: the fields of the view
/// without `id`.
fn form(connection: Connection) -> Value {
    let mut fields = serde_json::to_value(ConnectionView::new(None, connection)).unwrap();
    fields.as_object_mut().unwrap().remove("id");
    fields
}

/// A connection named `name` that needs nothing to be saved.
fn named(name: &str) -> Connection {
    // Nothing listens on this port: saving does not need the service.
    Connection {
        name: name.to_owned(),
        model: "llama".to_owned(),
        url: "http://127.0.0.1:9/v1/chat/completions".to_owned(),
        ..Connection::template()
    }
}

fn save(window: &TestWindow, id: Option<i64>, connection: Value) -> Result<Value, Value> {
    window.call("connection_form_save", json!({ "id": id, "connection": connection }))
}

#[test]
fn a_connection_is_saved_read_and_changed() {
    let window = logged_in_window();

    let saved = save(&window, None, form(named("Ollama"))).unwrap();
    assert_eq!(saved["notice"], "The connection was saved.");
    let id = saved["id"].as_i64().unwrap();

    let read = window.call("connection_form_get", json!({ "id": id })).unwrap();
    assert_eq!(read["id"], id);
    assert_eq!(read["name"], "Ollama");
    assert_eq!(read["answer"], "stream");

    save(&window, Some(id), form(named("Renamed"))).unwrap();
    assert_eq!(window.call("connection_form_get", json!({ "id": id })).unwrap()["name"], "Renamed");

    // The first connection is the active one.
    assert_eq!(window.call("connection_list_get", json!({})).unwrap()["active"], id);
}

#[test]
fn a_broken_field_is_named_for_the_page() {
    let window = logged_in_window();
    let outside = Connection { url: "http://10.0.0.5/v1".to_owned(), ..named("Lab") };

    let error = save(&window, None, form(outside)).unwrap_err();

    assert_eq!(error["kind"], "invalid");
    assert_eq!(error["field"], "url");
}

#[test]
fn an_unknown_answer_mode_or_field_is_refused() {
    let window = logged_in_window();

    let mut other = form(named("X"));
    other["answer"] = json!("other");
    assert_eq!(save(&window, None, other).unwrap_err()["kind"], "invalid");

    let mut extra = form(named("X"));
    extra["api_key"] = json!("sk-should-not-be-here");
    assert!(save(&window, None, extra).is_err());
}

#[test]
fn the_active_connection_is_chosen_and_deleted() {
    let window = logged_in_window();
    let first = save(&window, None, form(named("A"))).unwrap()["id"].clone();
    let second = save(&window, None, form(named("B"))).unwrap()["id"].clone();

    window.call("connection_form_activate", json!({ "id": second })).unwrap();
    assert_eq!(window.call("connection_list_get", json!({})).unwrap()["active"], second);
    assert_eq!(window.call("connection_form_activate", json!({ "id": 99 })).unwrap_err()["kind"], "invalid");

    window.call("connection_form_delete", json!({ "id": second })).unwrap();

    let list = window.call("connection_list_get", json!({})).unwrap();
    assert_eq!(list["active"], json!(null));
    assert_eq!(list["connections"], json!([{ "id": first, "name": "A" }]));
    assert_eq!(window.call("connection_form_delete", json!({ "id": second })).unwrap_err()["kind"], "invalid");
}

#[test]
fn a_new_connection_starts_from_the_template_and_the_presets() {
    let new = logged_in_window().call("connection_form_new", json!({})).unwrap();

    assert_eq!(new["template"]["id"], json!(null));
    assert_eq!(new["presets"][0]["id"], "local-gemma");
    assert_eq!(new["presets"][0]["fields"]["id"], json!(null));
}

#[test]
fn an_instruction_is_given_for_a_field_of_the_form() {
    let window = logged_in_window();
    let ask = |field: &str| {
        window.call("connection_form_field_help", json!({ "field": field, "connection": form(named("X")) }))
    };

    assert!(ask("text_path").unwrap().as_str().unwrap().contains("The field: Path to the text"));
    assert_eq!(ask("api_key").unwrap_err()["kind"], "invalid");
}

#[test]
fn nothing_works_without_a_login() {
    let window = logged_out_window();

    assert_eq!(window.call("connection_form_new", json!({})).unwrap_err()["kind"], "not_logged_in");
    assert_eq!(save(&window, None, form(named("X"))).unwrap_err()["kind"], "not_logged_in");
}
