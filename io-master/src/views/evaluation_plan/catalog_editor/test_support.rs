//! Help for the tests of the editor of a catalog: a window with the small
//! standard loaded and a catalog made from it; for the work of the AI, a
//! state with a fake AI service and a channel that collects the events.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::ai::api::connections;
use crate::ai::api::test_support::{FakeService, completions as connection_to};
use crate::app::test_support::{TestState, logged_in};
use crate::model::api_key::ApiKey;
use crate::test_support::standards::load;
use crate::views::evaluation_plan::catalog_editor::ai_builder::view::BuildEvent;
use crate::views::evaluation_plan::new_catalog::made;
use crate::views::test_support::{TestWindow, logged_in_window};

/// A window with "anna" logged in, the small standard and an empty
/// catalog made from it; returns the window, the catalog and the document.
pub fn with_catalog() -> (TestWindow, String, i64) {
    let window = logged_in_window();

    let (uuid, source) = {
        let session = window.session();
        let db = session.db();
        let source = load(&db, b"%PDF editor");

        (made::create(&db, "CC Part 3", "CC:2022", &[source]).unwrap().uuid, source)
    };

    (window, uuid, source)
}

/// Adds a group `id` at the top, as the dialog of a new object does.
pub fn add_group(window: &TestWindow, uuid: &str, id: &str) {
    let group = json!({ "id": id, "title": format!("Group {id}") });

    window.call("new_object_add_group", json!({ "uuid": uuid, "place": { "kind": "top" }, "group": group })).unwrap();
}

/// Adds a control `id` with a statement into group `group`.
pub fn add_control(window: &TestWindow, uuid: &str, group: &str, id: &str) -> Result<Value, Value> {
    let control = json!({ "id": id, "title": format!("Control {id}"),
        "parts": [{ "id": format!("{id}_smt"), "name": "statement", "prose": "The developer shall provide." }] });

    window.call(
        "new_object_add_control",
        json!({ "uuid": uuid, "place": { "kind": "group", "id": group }, "control": control }),
    )
}

/// "anna" logged in, her AI the fake `service`, the small standard loaded
/// and an empty catalog made from it; returns the state, the catalog and
/// the document.
pub async fn ready(service: &FakeService) -> (TestState, String, i64) {
    let test = logged_in().await;

    let (uuid, source) = {
        let session = test.state.session().unwrap();
        let db = session.db();
        let id = connections::create(&db, &connection_to(service)).unwrap().id;
        connections::set_key(&db, id, &ApiKey::new("sk-test".to_owned()).unwrap()).unwrap();
        let source = load(&db, b"%PDF building");

        (made::create(&db, "CC Part 3", "", &[source]).unwrap().uuid, source)
    };

    (test, uuid, source)
}

/// A channel that collects the events it gets, as JSON.
pub fn collector() -> (Arc<Mutex<Vec<Value>>>, Channel<BuildEvent>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);

    let channel = Channel::new(move |body: InvokeResponseBody| {
        sink.lock().unwrap().push(body.deserialize::<Value>().unwrap());
        Ok(())
    });

    (events, channel)
}
