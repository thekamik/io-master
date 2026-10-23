//! The element "scanner settings" of the view Devices
//! (page: `ui/src/views/devices/ScannerSettings.svelte`).
//!
//! ```text
//! action   command                 parameters                 data back
//! read     scanner_settings_get    (none)                     ScannerSettingsView
//! save     scanner_settings_save   all_types, types, columns  ScannerSettingsView, as stored
//! ```
//!
//! `ScannerSettingsView` holds the settings, their summary in one line,
//! and what can be chosen. After a save the page tells the table of
//! ports, which scans again (`../mod.rs`).

pub mod commands;
pub mod view;
