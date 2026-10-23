//! The command of the user menu.

use tauri::State;

use crate::app::state::AppState;

/// Logs out: ends the session of the evaluator.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn user_menu_log_out(state: State<'_, AppState>) {
    state.log_out();
}

#[cfg(test)]
mod tests {
    //! Unit tests of `user_menu/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::test_support::logged_in_window;

    #[test]
    fn the_logout_ends_the_session() {
        let window = logged_in_window();

        window.call("user_menu_log_out", json!({})).unwrap();

        assert_eq!(window.call("program_get", json!({})).unwrap()["user"], json!(null));
        assert_eq!(window.call("ports_table_scan", json!({})).unwrap_err()["kind"], "not_logged_in");
    }
}
