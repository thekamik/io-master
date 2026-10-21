//! Help for the tests of the editor of a catalog: a window with the small
//! standard loaded and a catalog made from it.

use serde_json::{Value, json};

use crate::test_support::standards::load;
use crate::views::plans::new_catalog::made;
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
