//! The settings of the scanner of serial ports, as two texts
//! (`hardware_tests/devices/scan_settings.rs` makes them and reads
//! them).
//!
//! **Where: the database of the evaluator, encrypted by `SQLCipher`**
//! (`db/user/database.rs`), in its table `settings` (`db/user/schema.rs`),
//! so the area has no tables of its own and no `schema.rs`:
//!
//! ```text
//! settings.name        settings.value
//! scan.port_types      "all", or base names: "ttyACM,ttyUSB"
//! scan.columns         keys of the columns: "path,type,device"
//! ```
//!
//! ```text
//! read.rs     both texts, None for one never stored
//! update.rs   both texts stored, in one transaction
//! ```
//!
//! Nothing is created or deleted on its own: the settings are made
//! with their first update and removed with the database.

pub mod read;
pub mod update;

/// The name of the setting with the kinds of port.
const PORT_TYPES: &str = "scan.port_types";

/// The name of the setting with the columns.
const COLUMNS: &str = "scan.columns";
