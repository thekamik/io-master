//! The settings of the scanner as they are stored, read for the element
//! "scanner settings" and for the scan of the table of ports.

use crate::db::error::DbError;
use crate::db::scan_settings as db_scan_settings;
use crate::db::user::database::UserDb;
use crate::hardware_tests::devices::scan_settings::ScanSettings;

/// The stored settings of the evaluator; the defaults for what was
/// never stored.
///
/// Called by: `scanner_settings_get` and `ports_table_scan` (`devices.rs`).
/// Calls: `db_scan_settings::read::texts`, `ScanSettings::from_stored` (the
/// rules of the settings).
pub fn read(db: &UserDb) -> Result<ScanSettings, DbError> {
    let stored = db_scan_settings::read::texts(db)?;

    Ok(ScanSettings::from_stored(stored.port_types.as_deref(), stored.columns.as_deref()))
}
