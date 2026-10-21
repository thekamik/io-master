//! Unit tests of `scan_settings.rs`.

use super::*;

/// The texts of a form.
fn texts(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_form_gives_the_kinds_and_the_columns_in_the_order_of_the_table() {
    let settings = ScanSettings::from_form(false, &texts(&["ttyACM", "ttyUSB"]), &texts(&["driver", "path"])).unwrap();

    assert_eq!(settings.filter, PortFilter::Types(vec![PortType::UsbAcm, PortType::UsbSerial]));
    assert_eq!(settings.columns, [PortProperty::Path, PortProperty::Driver]);
}

#[test]
fn an_empty_form_means_everything_with_the_default_columns() {
    assert_eq!(ScanSettings::from_form(false, &[], &[]).unwrap(), ScanSettings::default());
    assert_eq!(ScanSettings::from_form(true, &texts(&["ttyACM"]), &[]).unwrap(), ScanSettings::default());
}

#[test]
fn a_kind_without_a_name_in_this_version_is_kept() {
    let settings = ScanSettings::from_form(false, &texts(&["ttyRP"]), &[]).unwrap();

    assert_eq!(settings.filter, PortFilter::Types(vec![PortType::Other("ttyRP".to_owned())]));
}

#[test]
fn a_form_with_what_the_page_never_sends_is_refused() {
    let refused = [
        (texts(&[""]), Vec::new(), ScanSettingsError::BadBaseName(String::new())),
        (texts(&["tty/ACM"]), Vec::new(), ScanSettingsError::BadBaseName("tty/ACM".to_owned())),
        (Vec::new(), texts(&["path", "not_a_column"]), ScanSettingsError::UnknownColumn("not_a_column".to_owned())),
        (vec!["ttyS".to_owned(); MAX_PORT_TYPES + 1], Vec::new(), ScanSettingsError::TooManyTypes(MAX_PORT_TYPES + 1)),
        (Vec::new(), vec!["path".to_owned(); MAX_COLUMNS + 1], ScanSettingsError::TooManyColumns(MAX_COLUMNS + 1)),
    ];

    for (types, columns, error) in refused {
        assert_eq!(ScanSettings::from_form(false, &types, &columns), Err(error));
    }
}

#[test]
fn the_texts_are_read_back_as_they_were_stored() {
    let settings = ScanSettings::from_form(false, &texts(&["ttyACM", "ttyRP"]), &texts(&["path", "driver"])).unwrap();

    assert_eq!(settings.stored_port_types(), "ttyACM,ttyRP");
    assert_eq!(settings.stored_columns(), "path,driver");

    let read = ScanSettings::from_stored(Some(&settings.stored_port_types()), Some(&settings.stored_columns()));
    assert_eq!(read, settings);

    assert_eq!(ScanSettings::default().stored_port_types(), "all");
    assert_eq!(ScanSettings::from_stored(Some("all"), None), ScanSettings::default());
}

#[test]
fn nothing_stored_gives_the_defaults() {
    assert_eq!(ScanSettings::from_stored(None, None), ScanSettings::default());
}

#[test]
fn stored_texts_that_cannot_be_used_are_passed_over() {
    let settings = ScanSettings::from_stored(Some("ttyACM,,tty/x"), Some("removed_column,path,also_unknown"));
    assert_eq!(settings.filter, PortFilter::Types(vec![PortType::UsbAcm]));
    assert_eq!(settings.columns, [PortProperty::Path]);

    assert_eq!(ScanSettings::from_stored(Some(","), Some("nothing,known")), ScanSettings::default());
}
