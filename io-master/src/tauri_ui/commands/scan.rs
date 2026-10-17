//! The scan of serial ports with the settings of the evaluator.
//!
//! The scan only reads the descriptions of the ports and never
//! opens one (`hardware_tests/devices/devices.rs`).

use crate::business::hardware::scan_settings::ScanSettings;
use crate::hardware_tests::devices::devices::{self, PortType};
use crate::hardware_tests::devices::port_property::PortProperty;

use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::scan_view::ScanView;
use crate::tauri_ui::session::with_user_db;
use crate::tauri_ui::state::UiState;

/// Scans with the stored settings. A failed scan is shown in the
/// page (`ScanView::problem`), not as an error of the command.
pub async fn scan(state: &UiState) -> Result<ScanView, UiError> {
    with_user_db(state, |db| {
        let settings = ScanSettings::load(db)?;
        let outcome = devices::scan(&settings.filter).map_err(|error| format!("The ports cannot be scanned: {error}."));

        Ok(ScanView::new(&settings, outcome))
    })
    .await
}

/// Stores new settings and scans with them. Unknown column keys are
/// passed over; nothing chosen means the defaults (`ScanSettings::new`).
pub async fn save_scan_settings(
    state: &UiState,
    all_types: bool,
    types: Vec<String>,
    columns: Vec<String>,
) -> Result<ScanView, UiError> {
    let types = types.iter().filter(|name| !name.is_empty()).map(|name| PortType::from_base_name(name)).collect();
    let columns = columns.iter().filter_map(|key| PortProperty::from_key(key)).collect();
    let settings = ScanSettings::new(all_types, types, columns);

    with_user_db(state, move |db| Ok(settings.save(db)?)).await?;

    scan(state).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
