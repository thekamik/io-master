//! The settings of the scanner of an evaluator: which kinds of port
//! a scan looks for, and which properties the table of ports shows.
//!
//! They are kept in the encrypted database of the evaluator
//! (`db/user/`), as two settings of plain text:
//!
//! ```text
//! scan.port_types   "all", or base names: "ttyACM,ttyUSB"
//! scan.columns      keys of PortProperty: "path,type,device"
//! ```
//!
//! Reading is forgiving: an unknown column (for example one removed
//! by a later version) is passed over, and settings that cannot be
//! used at all fall back to the defaults, so a stored choice never
//! stops the page from working.

use crate::db::error::DbError;
use crate::db::user;
use crate::db::user::database::UserDb;

use super::devices::{PortFilter, PortType};
use super::port_property::PortProperty;

/// Name of the setting with the kinds of port.
const PORT_TYPES: &str = "scan.port_types";

/// Name of the setting with the columns.
const COLUMNS: &str = "scan.columns";

/// The stored text for "every kind of port".
const ALL: &str = "all";

/// The settings of the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanSettings {
    /// Which ports a scan reports.
    pub filter: PortFilter,
    /// Which properties the table shows, in this order. Never
    /// empty.
    pub columns: Vec<PortProperty>,
}

impl Default for ScanSettings {
    /// Every port, with the default columns.
    fn default() -> ScanSettings {
        ScanSettings { filter: PortFilter::All, columns: PortProperty::DEFAULT.to_vec() }
    }
}

impl ScanSettings {
    /// Settings from a choice made in a form: the kinds of port (or
    /// "all") and the columns. Nothing chosen means every kind of
    /// port, and the default columns: an empty table is never what
    /// was meant.
    pub fn new(all_types: bool, types: Vec<PortType>, columns: Vec<PortProperty>) -> ScanSettings {
        let filter = if all_types || types.is_empty() { PortFilter::All } else { PortFilter::Types(types) };

        let columns = if columns.is_empty() {
            PortProperty::DEFAULT.to_vec()
        } else {
            // In the order of the table, whatever order they came in.
            PortProperty::ALL.into_iter().filter(|property| columns.contains(property)).collect()
        };

        ScanSettings { filter, columns }
    }

    /// Reads the settings of an evaluator; the defaults for what was
    /// never stored.
    pub fn load(db: &UserDb) -> Result<ScanSettings, DbError> {
        let defaults = ScanSettings::default();

        let filter = match user::read::setting(db, PORT_TYPES)? {
            None => defaults.filter,
            Some(text) if text == ALL => PortFilter::All,
            Some(text) => {
                let types: Vec<PortType> =
                    text.split(',').filter(|name| !name.is_empty()).map(PortType::from_base_name).collect();

                if types.is_empty() { PortFilter::All } else { PortFilter::Types(types) }
            }
        };

        let columns = match user::read::setting(db, COLUMNS)? {
            None => defaults.columns,
            Some(text) => {
                let columns: Vec<PortProperty> = text.split(',').filter_map(PortProperty::from_key).collect();

                if columns.is_empty() { defaults.columns } else { columns }
            }
        };

        Ok(ScanSettings { filter, columns })
    }

    /// Stores the settings of an evaluator.
    pub fn save(&self, db: &UserDb) -> Result<(), DbError> {
        let types = match &self.filter {
            PortFilter::All => ALL.to_owned(),
            PortFilter::Types(types) => types.iter().map(PortType::base_name).collect::<Vec<_>>().join(","),
        };

        let columns = self.columns.iter().map(|column| column.key()).collect::<Vec<_>>().join(",");

        user::update::set_setting(db, PORT_TYPES, &types)?;
        user::update::set_setting(db, COLUMNS, &columns)
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "scan_settings_tests.rs"]
mod tests;
