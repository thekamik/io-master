//! The commands of the settings of the scanner.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::scan_settings as db_scan_settings;
use crate::hardware_tests::devices::scan_settings::ScanSettings;

use super::super::stored_settings;
use super::view::ScannerSettingsView;

/// The stored settings of the evaluator, and what can be chosen.
///
/// `async`: on a thread of the runtime of Tauri, not of the window
/// (`ports_table/commands.rs`).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn scanner_settings_get(state: State<'_, AppState>) -> Result<ScannerSettingsView, AppError> {
    let session = state.session()?;
    let settings = stored_settings::read(&session.db())?;

    Ok(ScannerSettingsView::new(&settings))
}

/// Stores the settings chosen in the form: all kinds of port, or the
/// kinds named in `types` (base names: `"ttyACM"`); the columns named by
/// their keys in `columns` (`"path"`). A name the page never offers is
/// refused (`AppError::Invalid`) and nothing is stored. Gives back the
/// settings as they were stored.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn scanner_settings_save(
    state: State<'_, AppState>,
    all_types: bool,
    types: Vec<String>,
    columns: Vec<String>,
) -> Result<ScannerSettingsView, AppError> {
    let session = state.session()?;
    let settings = ScanSettings::from_form(all_types, &types, &columns)?;

    db_scan_settings::update::texts(&session.db(), &settings.stored_port_types(), &settings.stored_columns())?;

    Ok(ScannerSettingsView::new(&settings))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `scanner_settings/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::views::test_support::{logged_in_window, logged_out_window};

    #[test]
    fn nothing_stored_gives_the_defaults_and_every_choice() {
        let window = logged_in_window();

        let settings = window.call("scanner_settings_get", json!({})).unwrap();

        assert_eq!(settings["all_types"], true);
        assert_eq!(settings["columns"], json!(["path", "type", "device", "serial_number", "access"]));
        assert_eq!(settings["summary"], "all kinds of port · 5 columns");
        assert_eq!(settings["choices"]["types"][0], "ttyS");
        assert_eq!(settings["choices"]["columns"].as_array().unwrap().len(), 19);
    }

    #[test]
    fn saved_settings_are_given_back_as_stored_and_kept() {
        let window = logged_in_window();
        let chosen = json!({ "allTypes": false, "types": ["ttyACM", "ttyRP"], "columns": ["driver", "path"] });

        let saved = window.call("scanner_settings_save", chosen).unwrap();

        assert_eq!(saved["all_types"], false);
        assert_eq!(saved["types"], json!(["ttyACM", "ttyRP"]));
        assert_eq!(saved["columns"], json!(["path", "driver"]));
        assert_eq!(saved["summary"], "ttyACM, ttyRP · 2 columns");
        // A kind this version has no name for stays among the choices.
        assert!(saved["choices"]["types"].as_array().unwrap().contains(&json!("ttyRP")));

        assert_eq!(window.call("scanner_settings_get", json!({})).unwrap(), saved);
    }

    #[test]
    fn what_the_page_never_sends_is_refused_and_nothing_is_stored() {
        let window = logged_in_window();
        let wrong = json!({ "allTypes": true, "types": [], "columns": ["path", "not_a_column"] });

        let error = window.call("scanner_settings_save", wrong).unwrap_err();

        assert_eq!(error["kind"], "invalid");
        assert_eq!(window.call("scanner_settings_get", json!({})).unwrap()["all_types"], true);
    }

    #[test]
    fn the_settings_need_a_login() {
        let window = logged_out_window();

        assert_eq!(window.call("scanner_settings_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }
}
