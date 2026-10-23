//! The element "table of ports" of the view Devices
//! (page: `ui/src/views/devices/PortsTable.svelte`).
//!
//! ```text
//! action   command            parameters   data back
//! scan     ports_table_scan   (none)       PortsTableView: the columns, a row per port, or why the scan failed
//! ```
//!
//! The scan uses the stored settings of the evaluator (`../stored_settings.rs`);
//! the page asks for it when the view opens, when "Scan again" is
//! pressed, and after the settings are saved.

pub mod commands;
pub mod view;
