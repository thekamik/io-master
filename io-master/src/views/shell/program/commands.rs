//! The command of the state of the program.

use tauri::State;

use crate::app::state::AppState;

use super::view::ProgramView;

/// The state of the program: whether it is installed, its role, who is
/// logged in, the pages, the warnings, the version and the folder.
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn program_get(state: State<'_, AppState>) -> ProgramView {
    ProgramView::new(state.role(), state.current_user(), &state.data_dir)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `program/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::test_support::{first_start_window, logged_in_window};

    #[test]
    fn at_the_first_start_the_program_is_not_installed() {
        let program = first_start_window().call("program_get", json!({})).unwrap();

        assert_eq!(program["installed"], false);
        assert_eq!(program["role"], json!(null));
        assert_eq!(program["pages"], json!([]));
        assert_eq!(program["warnings"], json!([]));
    }

    #[test]
    fn a_workstation_shows_its_pages_its_warnings_and_its_user() {
        let program = logged_in_window().call("program_get", json!({})).unwrap();

        assert_eq!(program["user"], "anna");
        assert_eq!(program["pages"].as_array().unwrap().len(), 5);
        assert_eq!(program["pages"][1], json!({ "id": "evaluation_plan", "title": "Evaluation plan" }));
        assert_eq!(program["pages"][2], json!({ "id": "evaluation_lab", "title": "Evaluation lab" }));
        assert_eq!(program["pages"][3], json!({ "id": "devices", "title": "Devices" }));
        assert_eq!(program["warnings"].as_array().unwrap().len(), 2);
        assert_ne!(program["version"], "");
    }
}
