//! Unit tests of `setup_form/commands.rs`, through the IPC of Tauri.

use serde_json::{Value, json};

use crate::app::test_support::PASSWORD;
use crate::views::test_support::first_start_window;

/// The form of the setup.
fn form(name: &str, password: &str, repeat: &str, role: &str, confirmed: bool) -> Value {
    json!({ "name": name, "password": password, "passwordRepeat": repeat, "role": role, "roleConfirmed": confirmed })
}

#[test]
fn the_setup_installs_and_logs_the_user_in_once() {
    let window = first_start_window();

    window.call("setup_form_install", form(" anna ", PASSWORD, PASSWORD, "workstation", true)).unwrap();

    let program = window.call("program_get", json!({})).unwrap();
    assert_eq!(program["installed"], true);
    assert_eq!(program["role"], "workstation");
    assert_eq!(program["user"], "anna");

    let again = window.call("setup_form_install", form("jan", PASSWORD, PASSWORD, "master", true)).unwrap_err();
    assert_eq!(again["kind"], "already_installed");
}

#[test]
fn an_incomplete_setup_installs_nothing() {
    let window = first_start_window();
    let cases = [
        form("anna", PASSWORD, "other password", "workstation", true),
        form("anna", PASSWORD, PASSWORD, "", true),
        form("anna", PASSWORD, PASSWORD, "workstation", false),
        form("anna", "short", "short", "workstation", true),
        form("", PASSWORD, PASSWORD, "workstation", true),
    ];

    for case in cases {
        let error = window.call("setup_form_install", case.clone()).unwrap_err();

        assert_eq!(error["kind"], "invalid", "{case}");
    }

    assert_eq!(window.call("program_get", json!({})).unwrap()["installed"], false);
}
