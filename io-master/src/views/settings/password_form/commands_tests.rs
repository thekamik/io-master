//! Unit tests of `password_form/commands.rs`, through the IPC of Tauri.

use serde_json::json;

use crate::app::test_support::PASSWORD;
use crate::views::test_support::logged_in_window;

#[test]
fn the_password_is_changed_with_the_current_one() {
    let window = logged_in_window();
    let new = "another long password";
    let change = |current: &str, password: &str, repeat: &str| {
        window
            .call("password_form_change", json!({ "current": current, "password": password, "passwordRepeat": repeat }))
    };

    let wrong = change("not the password", new, new).unwrap_err();
    assert_eq!(wrong["kind"], "invalid");
    assert!(wrong["message"].as_str().unwrap().contains("current password"));

    assert_eq!(change(PASSWORD, new, "different").unwrap_err()["kind"], "invalid");

    change(PASSWORD, new, new).unwrap();

    // The new password opens the database; the old one no longer does.
    window.call("user_menu_log_out", json!({})).unwrap();
    assert!(window.call("login_form_log_in", json!({ "name": "anna", "password": PASSWORD })).is_err());
    window.call("login_form_log_in", json!({ "name": "anna", "password": new })).unwrap();
}
