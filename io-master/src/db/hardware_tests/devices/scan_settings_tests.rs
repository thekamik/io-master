//! Unit tests of `scan_settings.rs`.

use super::*;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::random::random::fill_random;
use crate::tpm::test_support::TempDir;

/// A new database of an evaluator in a temporary directory.
fn database(directory: &TempDir) -> UserDb {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    user_db::create(directory.path(), 1, &key).unwrap()
}

#[test]
fn nothing_stored_gives_the_defaults() {
    let directory = TempDir::new();

    assert_eq!(ScanSettings::load(&database(&directory)).unwrap(), ScanSettings::default());
}

#[test]
fn stored_settings_are_read_back() {
    let directory = TempDir::new();
    let db = database(&directory);

    let settings = ScanSettings::new(
        false,
        vec![PortType::UsbAcm, PortType::UsbSerial],
        vec![PortProperty::Path, PortProperty::Driver],
    );

    settings.save(&db).unwrap();

    assert_eq!(ScanSettings::load(&db).unwrap(), settings);
    assert_eq!(db.setting("scan.port_types").unwrap().as_deref(), Some("ttyACM,ttyUSB"));
    assert_eq!(db.setting("scan.columns").unwrap().as_deref(), Some("path,driver"));
}

#[test]
fn all_types_and_unknown_types_are_kept() {
    let directory = TempDir::new();
    let db = database(&directory);

    ScanSettings::new(true, vec![PortType::UsbAcm], PortProperty::DEFAULT.to_vec()).save(&db).unwrap();
    assert_eq!(ScanSettings::load(&db).unwrap().filter, PortFilter::All);

    // A kind of port this version has no name for.
    let other = PortType::Other("ttyRP".to_owned());
    ScanSettings::new(false, vec![other.clone()], PortProperty::DEFAULT.to_vec()).save(&db).unwrap();
    assert_eq!(ScanSettings::load(&db).unwrap().filter, PortFilter::Types(vec![other]));
}

#[test]
fn an_empty_choice_means_everything_with_the_default_columns() {
    let settings = ScanSettings::new(false, Vec::new(), Vec::new());

    assert_eq!(settings, ScanSettings::default());
}

#[test]
fn columns_keep_the_order_of_the_table() {
    let settings = ScanSettings::new(true, Vec::new(), vec![PortProperty::Driver, PortProperty::Path]);

    assert_eq!(settings.columns, [PortProperty::Path, PortProperty::Driver]);
}

#[test]
fn unusable_stored_settings_fall_back_to_the_defaults() {
    let directory = TempDir::new();
    let db = database(&directory);

    db.set_setting("scan.columns", "removed_column,path,also_unknown").unwrap();
    assert_eq!(ScanSettings::load(&db).unwrap().columns, [PortProperty::Path]);

    db.set_setting("scan.columns", "nothing,known").unwrap();
    db.set_setting("scan.port_types", ",").unwrap();

    assert_eq!(ScanSettings::load(&db).unwrap(), ScanSettings::default());
}
