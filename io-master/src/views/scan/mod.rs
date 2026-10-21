//! The view Scan: the serial ports of the computer and the settings of
//! the scanner (page: `ui/src/views/scan/`).
//!
//! ```text
//! element            folder               commands
//! the table of ports ports_table/         ports_table_scan
//! the settings       scanner_settings/    scanner_settings_get, scanner_settings_save
//! ```
//!
//! The flow of data. Each element reads its own data; when the settings
//! are saved, the page of the view tells the table, which scans again
//! with the new settings:
//!
//! ```text
//! settings ──save(all_types, types, columns)──► scanner_settings_save ──► db: the texts stored
//!    ▲                                                                       │
//!    └────────────────── ScannerSettingsView ────────────────────────────────┘
//!    └── "saved" ──► the view ──► table ──scan()──► ports_table_scan ──► db: the texts read
//!                                   ▲                                    hardware_tests: the scan
//!                                   └────────────── PortsTableView ──────────┘
//! ```
//!
//! The scan only reads the descriptions of the ports; it never opens
//! one.

pub mod column_view;
pub mod ports_table;
pub mod scanner_settings;
pub mod stored_settings;
