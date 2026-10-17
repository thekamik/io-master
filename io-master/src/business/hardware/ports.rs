//! The serial ports as a resource (step 5 of `docs/todo.md`, moved in
//! step 40): the scan with the settings of the evaluator. The scan only
//! reads the descriptions of the ports and never opens one
//! (`hardware_tests/devices/serial_ports.rs`).
//!
//! ```text
//! scan()            GET   /ports            with the stored settings
//! save_settings()   PUT   /ports/settings   the kinds of port and the columns
//! ```

use crate::db::error::DbError;
use crate::db::user::database::UserDb;
use crate::hardware_tests::devices::port_property::PortProperty;
use crate::hardware_tests::devices::serial_ports::{self as devices, PortType, SerialPortInfo};

use super::scan_settings::ScanSettings;

/// A scan: the settings it used, and the ports, or why the ports could
/// not be scanned (a sentence for the evaluator; shown in the page, not
/// an error of the work).
#[derive(Debug)]
pub struct Scan {
    pub settings: ScanSettings,
    pub ports: Result<Vec<SerialPortInfo>, String>,
}

/// Scans with the stored settings.
pub fn scan(db: &UserDb) -> Result<Scan, DbError> {
    let settings = ScanSettings::load(db)?;
    let ports = devices::scan(&settings.filter).map_err(|error| format!("The ports cannot be scanned: {error}."));

    Ok(Scan { settings, ports })
}

/// Stores new settings: all kinds of port, or the kinds named in
/// `types`; the columns named by their keys in `columns`. Unknown names
/// and keys are passed over; nothing chosen means the defaults
/// (`ScanSettings::new`).
pub fn save_settings(db: &UserDb, all_types: bool, types: &[String], columns: &[String]) -> Result<(), DbError> {
    let types = types.iter().filter(|name| !name.is_empty()).map(|name| PortType::from_base_name(name)).collect();
    let columns = columns.iter().filter_map(|key| PortProperty::from_key(key)).collect();

    ScanSettings::new(all_types, types, columns).save(db)
}
