//! Unit tests of `object_quotes/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::views::evaluation_plan::catalog_editor::test_support::{add_control, add_group, with_catalog};

#[test]
fn quotes_are_looked_for_on_their_pages_and_go_with_their_object() {
    let (window, uuid, source) = with_catalog();
    add_group(&window, &uuid, "ADV");
    add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();
    let add = |text: &str| {
        window.call(
            "object_quotes_add",
            json!({ "uuid": uuid, "objectId": "ADV_FSP.4", "source": source, "page": 2, "text": text }),
        )
    };

    add("The developer shall provide a functional specification.").unwrap();
    add("The evaluator shall dance.").unwrap();
    assert_eq!(add("").unwrap_err()["kind"], "invalid");

    let quotes = window.call("object_quotes_get", json!({ "uuid": uuid, "objectId": "ADV_FSP.4" })).unwrap();
    assert_eq!(quotes["quotes"][0]["found"], true, "across the divided word");
    assert_eq!(quotes["quotes"][1]["found"], false, "made up");
    assert_eq!(quotes["quotes"][0]["document"], "Common Criteria — Part 3");
    assert_eq!(quotes["first_source"], source);

    // A document of a standard that a quote names stays.
    assert_eq!(window.call("standard_documents_delete", json!({ "id": source })).unwrap_err()["kind"], "invalid");

    let id = quotes["quotes"][1]["id"].clone();
    window.call("object_quotes_delete", json!({ "id": id })).unwrap();

    window.call("object_form_delete", json!({ "uuid": uuid, "id": "ADV" })).unwrap();
    let after = window.call("object_quotes_get", json!({ "uuid": uuid, "objectId": "ADV_FSP.4" })).unwrap();
    assert_eq!(after["quotes"], json!([]));
}

#[test]
fn a_quote_of_no_object_is_refused() {
    let (window, uuid, source) = with_catalog();

    let error = window
        .call(
            "object_quotes_add",
            json!({ "uuid": uuid, "objectId": "NONE", "source": source, "page": 1, "text": "Part 3" }),
        )
        .unwrap_err();

    assert_eq!(error["kind"], "invalid");
}
