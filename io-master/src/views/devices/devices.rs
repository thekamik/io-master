//! The entry of the view Devices: every command the page calls for it
//! (page: `ui/src/views/devices/`, its commands in
//! `ui/src/lib/api/devices.ts`). The serial ports of the computer and the
//! settings of the scanner; the scan only reads the descriptions of the
//! ports and never opens one. The flow of data: `docs/architecture.md`,
//! section 12.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::db::scan_settings as db_scan_settings;
use crate::hardware_tests::devices::scan_settings::ScanSettings;
use crate::hardware_tests::devices::serial_ports;

use super::ports_table::view::PortsTableView;
use super::scanner_settings::stored;
use super::scanner_settings::view::ScannerSettingsView;

// ── When the view opens ──────────────────────────────────────────────

/// The stored settings of the scanner, and what can be chosen.
///
/// Called by the page: `ScannerSettings.svelte`, when the view opens
/// (`devices.scannerSettingsGet()`).
/// Calls: `stored::read` (the texts of the database, read into
/// settings), `ScannerSettingsView::new`.
///
/// `async`: on a thread of the runtime of Tauri, not of the window, which
/// keeps answering meanwhile (the database takes milliseconds).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn scanner_settings_get(state: State<'_, AppState>) -> Result<ScannerSettingsView, AppError> {
    let session = state.session()?;
    let settings = stored::read(&session.db())?;

    Ok(ScannerSettingsView::new(&settings))
}

/// Scans the serial ports with the stored settings of the evaluator. A
/// scan that fails is shown in the table (`PortsTableView::problem`), not
/// as an error of the command; the command fails only when nobody is
/// logged in or the database fails.
///
/// Called by the page: `PortsTable.svelte`, when the view opens, when
/// "Scan again" is pressed, and after the settings are saved
/// (`devices.portsTableScan()`).
/// Calls: `stored::read`, `serial_ports::scan` (the descriptions
/// of the ports), `PortsTableView::new`.
///
/// `async`: as `scanner_settings_get`; the database is held only while the
/// settings are read, not during the scan.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ports_table_scan(state: State<'_, AppState>) -> Result<PortsTableView, AppError> {
    let session = state.session()?;
    let settings = stored::read(&session.db())?;
    let outcome = serial_ports::scan(&settings.filter);

    PortsTableView::new(&settings.columns, outcome)
}

// ── The actions of the elements ──────────────────────────────────────

/// Stores the settings chosen in the form: all kinds of port, or the
/// kinds named in `types` (base names: `"ttyACM"`); the columns named by
/// their keys in `columns` (`"path"`). A name the page never offers is
/// refused (`AppError::Invalid`) and nothing is stored. Gives back the
/// settings as they were stored; the page then tells the table, which
/// scans again.
///
/// Called by the page: `ScannerSettings.svelte`, "Save"
/// (`devices.scannerSettingsSave(...)`).
/// Calls: `ScanSettings::from_form` (the rules of the settings),
/// `db_scan_settings::update::texts`, `ScannerSettingsView::new`.
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
    //! Tests of the commands of the view Devices, called as the page calls
    //! them: by name, through the IPC of Tauri. The scan reads the real
    //! ports of the computer; the tests check the columns and the
    //! settings, not which ports exist.

    use serde_json::{Value, json};

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
}
