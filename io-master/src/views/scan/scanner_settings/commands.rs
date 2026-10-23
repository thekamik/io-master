//! The commands of the settings of the scanner.

use tauri::State;

use crate::app::error::UiError;
use crate::app::state::UiState;
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
pub fn scanner_settings_get(state: State<'_, UiState>) -> Result<ScannerSettingsView, UiError> {
    let session = state.session()?;
    let settings = stored_settings::read(&session.db())?;

    Ok(ScannerSettingsView::new(&settings))
}

/// Stores the settings chosen in the form: all kinds of port, or the
/// kinds named in `types` (base names: `"ttyACM"`); the columns named by
/// their keys in `columns` (`"path"`). A name the page never offers is
/// refused (`UiError::Invalid`) and nothing is stored. Gives back the
/// settings as they were stored.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn scanner_settings_save(
    state: State<'_, UiState>,
    all_types: bool,
    types: Vec<String>,
    columns: Vec<String>,
) -> Result<ScannerSettingsView, UiError> {
    let session = state.session()?;
    let settings = ScanSettings::from_form(all_types, &types, &columns)?;

    db_scan_settings::update::texts(&session.db(), &settings.stored_port_types(), &settings.stored_columns())?;

    Ok(ScannerSettingsView::new(&settings))
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
