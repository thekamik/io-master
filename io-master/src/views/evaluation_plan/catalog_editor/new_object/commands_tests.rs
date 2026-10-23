//! Unit tests of `new_object/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::views::evaluation_plan::catalog_editor::test_support::{add_control, add_group, with_catalog};

#[test]
fn an_id_is_used_once_and_a_place_must_exist() {
    let (window, uuid, _) = with_catalog();
    add_group(&window, &uuid, "ADV");
    add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

    assert_eq!(add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap_err()["kind"], "invalid");
    assert_eq!(add_control(&window, &uuid, "ATE", "ATE_IND.2").unwrap_err()["kind"], "invalid");

    let place = json!({ "kind": "elsewhere" });
    let group = json!({ "id": "X", "title": "X" });
    let error = window.call("new_object_add_group", json!({ "uuid": uuid, "place": place, "group": group }));
    assert_eq!(error.unwrap_err()["kind"], "invalid");
}

#[test]
fn an_enhancement_goes_into_its_control() {
    let (window, uuid, _) = with_catalog();
    add_group(&window, &uuid, "ADV");
    add_control(&window, &uuid, "ADV", "ADV_FSP.4").unwrap();

    let enhancement = json!({ "id": "ADV_FSP.4.a", "title": "More" });
    let place = json!({ "kind": "control", "id": "ADV_FSP.4" });
    window.call("new_object_add_control", json!({ "uuid": uuid, "place": place, "control": enhancement })).unwrap();

    let tree = window.call("catalog_tree_get", json!({ "uuid": uuid })).unwrap();
    assert_eq!(tree["rows"][2]["id"], "ADV_FSP.4.a");
    assert_eq!(tree["rows"][2]["level"], 2);
}
