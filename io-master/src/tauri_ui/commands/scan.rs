//! The scan of serial ports with the settings of the evaluator
//! (`business/hardware/ports.rs`).

use crate::business::hardware::ports;

use crate::tauri_ui::error::UiError;
use crate::tauri_ui::serializer::scan_view::ScanView;
use crate::tauri_ui::session::with_user_db;
use crate::tauri_ui::state::UiState;

/// Scans with the stored settings (`business/hardware/ports.rs`). A
/// failed scan is shown in the page (`ScanView::problem`), not as an
/// error of the command.
pub async fn scan(state: &UiState) -> Result<ScanView, UiError> {
    with_user_db(state, |db| {
        let scan = ports::scan(db)?;

        Ok(ScanView::new(&scan.settings, scan.ports))
    })
    .await
}

/// Stores new settings and scans with them.
pub async fn save_scan_settings(
    state: &UiState,
    all_types: bool,
    types: Vec<String>,
    columns: Vec<String>,
) -> Result<ScanView, UiError> {
    with_user_db(state, move |db| Ok(ports::save_settings(db, all_types, &types, &columns)?)).await?;

    scan(state).await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
