//! Unit tests of `ports_table/commands.rs`, through the IPC of Tauri. The
//! scan reads the real ports of the computer; the tests check the
//! columns and the settings, not which ports exist.

use serde_json::{Value, json};

use crate::views::test_support::{logged_in_window, logged_out_window};

#[test]
fn the_scan_uses_the_default_columns_when_nothing_is_stored() {
    let window = logged_in_window();

    let table = window.call("ports_table_scan", json!({})).unwrap();

    assert_eq!(table["columns"][0], json!({ "key": "path", "title": "Port" }));
    assert_eq!(table["columns"].as_array().unwrap().len(), 5);
    assert!(table["rows"].is_array());
}

#[test]
fn the_scan_uses_the_stored_settings() {
    let window = logged_in_window();
    let settings = json!({ "allTypes": false, "types": ["ttyACM"], "columns": ["driver", "path"] });
    window.call("scanner_settings_save", settings).unwrap();

    let table = window.call("ports_table_scan", json!({})).unwrap();
    let keys: Vec<&Value> = table["columns"].as_array().unwrap().iter().map(|column| &column["key"]).collect();

    assert_eq!(keys, [&json!("path"), &json!("driver")]);

    for row in table["rows"].as_array().unwrap() {
        assert_eq!(row.as_array().unwrap().len(), 2);
    }
}

#[test]
fn the_scan_needs_a_login() {
    let window = logged_out_window();

    assert_eq!(window.call("ports_table_scan", json!({})).unwrap_err()["kind"], "not_logged_in");
}
