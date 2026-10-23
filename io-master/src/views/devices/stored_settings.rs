//! The settings of the scanner as they are stored: read by both
//! elements of the view, so here, once.

use crate::db::error::DbError;
use crate::db::scan_settings as db_scan_settings;
use crate::db::user::database::UserDb;
use crate::hardware_tests::devices::scan_settings::ScanSettings;

/// The stored settings of the evaluator; the defaults for what was
/// never stored.
pub fn read(db: &UserDb) -> Result<ScanSettings, DbError> {
    let stored = db_scan_settings::read::texts(db)?;

    Ok(ScanSettings::from_stored(stored.port_types.as_deref(), stored.columns.as_deref()))
}
