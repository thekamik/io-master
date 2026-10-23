//! The commands of the table of ports.

use tauri::State;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::hardware_tests::devices::serial_ports;

use super::super::stored_settings;
use super::view::PortsTableView;

/// Scans the serial ports with the stored settings of the evaluator.
/// A scan that fails is shown in the table (`PortsTableView::problem`),
/// not as an error of the command; the command fails only when nobody
/// is logged in or the database fails.
///
/// `async` in the attribute runs the function on a thread of the
/// runtime of Tauri instead of the thread of the window, which keeps
/// answering meanwhile (the database and the scan take milliseconds).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn ports_table_scan(state: State<'_, AppState>) -> Result<PortsTableView, AppError> {
    let session = state.session()?;

    // The database is held only while the settings are read, not
    // during the scan.
    let settings = stored_settings::read(&session.db())?;
    let outcome = serial_ports::scan(&settings.filter);

    PortsTableView::new(&settings.columns, outcome)
}

#[cfg(test)]
mod tests {
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
}
