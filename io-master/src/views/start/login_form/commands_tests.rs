//! Unit tests of `login_form/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::tauri_ui::test_support::PASSWORD;
use crate::views::test_support::{first_start_window, logged_out_window};

#[test]
fn the_right_password_logs_in() {
    let window = logged_out_window();

    window.call("login_form_log_in", json!({ "name": " anna ", "password": PASSWORD })).unwrap();

    assert_eq!(window.call("program_get", json!({})).unwrap()["user"], "anna");
}

#[test]
fn a_wrong_password_and_an_unknown_name_are_refused_alike() {
    let window = logged_out_window();

    let wrong = window.call("login_form_log_in", json!({ "name": "anna", "password": "wrong password 1" }));
    let unknown = window.call("login_form_log_in", json!({ "name": "nobody", "password": PASSWORD }));

    assert_eq!(wrong.unwrap_err(), unknown.unwrap_err());
    assert_eq!(window.call("program_get", json!({})).unwrap()["user"], json!(null));
}

#[test]
fn nobody_logs_in_before_the_setup() {
    let window = first_start_window();

    let error = window.call("login_form_log_in", json!({ "name": "anna", "password": PASSWORD })).unwrap_err();

    assert_eq!(error["kind"], "not_installed");
}
