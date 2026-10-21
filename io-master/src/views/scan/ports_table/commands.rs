//! The commands of the table of ports.

use tauri::State;

use crate::hardware_tests::devices::serial_ports;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

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
pub fn ports_table_scan(state: State<'_, UiState>) -> Result<PortsTableView, UiError> {
    let session = state.session()?;

    // The database is held only while the settings are read, not
    // during the scan.
    let settings = stored_settings::read(&session.db())?;
    let outcome = serial_ports::scan(&settings.filter);

    PortsTableView::new(&settings.columns, outcome)
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
