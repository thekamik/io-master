//! Unit tests of `scan_page.rs`: turning the form into settings, and
//! the HTML of the form and the table. The routes are tested in
//! `web_ui_tests.rs`.

use super::*;

use std::collections::BTreeMap;

/// The fields of a form, as the browser sends them.
fn form(fields: &[(&str, &str)]) -> Vec<(String, String)> {
    fields.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
}

/// A port with a few properties.
fn port(name: &str) -> SerialPortInfo {
    SerialPortInfo {
        name: name.to_owned(),
        port_type: PortType::of_port(name),
        path: format!("/dev/{name}"),
        device_number: "166:0".to_owned(),
        sysfs_path: "/sys/devices/x".to_owned(),
        driver: Some("cdc_acm".to_owned()),
        bus: None,
        usb: None,
        links: vec!["/dev/serial/a".to_owned(), "/dev/serial/b".to_owned()],
        access: None,
        udev: BTreeMap::new(),
    }
}

#[test]
fn ticked_boxes_become_settings() {
    let settings = settings_from_form(&form(&[
        ("type", "ttyACM"),
        ("type", "ttyUSB"),
        ("column", "path"),
        ("column", "driver"),
    ]));

    assert_eq!(settings.filter, PortFilter::Types(vec![PortType::UsbAcm, PortType::UsbSerial]));
    assert_eq!(settings.columns, [PortProperty::Path, PortProperty::Driver]);
}

#[test]
fn all_kinds_wins_over_single_kinds() {
    let settings = settings_from_form(&form(&[("type", "all"), ("type", "ttyACM"), ("column", "path")]));

    assert_eq!(settings.filter, PortFilter::All);
}

#[test]
fn an_empty_form_gives_the_defaults_and_unknown_columns_are_dropped() {
    assert_eq!(settings_from_form(&[]), ScanSettings::default());

    let settings = settings_from_form(&form(&[("column", "not_a_column"), ("column", "bus")]));
    assert_eq!(settings.columns, [PortProperty::Bus]);
}

#[test]
fn the_form_shows_the_stored_choice() {
    let settings = ScanSettings::new(false, vec![PortType::UsbAcm], vec![PortProperty::Driver]);
    let html = settings_form(&settings);

    assert!(html.contains("value=\"ttyACM\" checked"));
    assert!(!html.contains("value=\"ttyUSB\" checked"));
    assert!(!html.contains("value=\"all\" checked"));
    assert!(html.contains("value=\"driver\" checked"));
    assert!(!html.contains("value=\"path\" checked"));

    // Every kind and every column can be chosen.
    assert!(html.contains("value=\"rfcomm\""));
    assert!(html.contains("value=\"sysfs_path\""));
}

#[test]
fn a_chosen_unknown_kind_stays_in_the_form() {
    let other = PortType::Other("ttyRP".to_owned());
    let settings = ScanSettings::new(false, vec![other], PortProperty::DEFAULT.to_vec());

    assert!(settings_form(&settings).contains("value=\"ttyRP\" checked"));
}

#[test]
fn the_table_has_the_chosen_columns() {
    let html = port_table(&[port("ttyACM0"), port("ttyACM1")], &[PortProperty::Path, PortProperty::Links]);

    assert!(html.contains("2 ports found."));
    assert!(html.contains("<th>Port</th><th>Stable names</th>"));
    assert!(html.contains("<td>/dev/ttyACM0</td>"));
    assert!(html.contains("/dev/serial/a<br>/dev/serial/b"));
    assert!(!html.contains("<th>Driver</th>"));
}

#[test]
fn values_from_the_system_cannot_inject_html() {
    let mut strange = port("ttyACM0");
    strange.driver = Some("<script>x</script>".to_owned());

    let html = port_table(&[strange], &[PortProperty::Driver]);

    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
}

#[test]
fn no_ports_is_said_in_words() {
    assert!(port_table(&[], &[PortProperty::Path]).contains("No serial ports"));
}

#[test]
fn the_settings_are_summed_up_in_one_line() {
    assert_eq!(summary(&ScanSettings::default()), "all kinds of port · 5 columns");

    let settings = ScanSettings::new(false, vec![PortType::UsbAcm, PortType::UsbSerial], vec![PortProperty::Path]);
    assert_eq!(summary(&settings), "ttyACM, ttyUSB · 1 column");
}

#[test]
fn the_result_comes_after_the_folded_settings() {
    let html = content(&ScanSettings::default(), &Outcome::Ports(vec![port("ttyACM0")]));

    let settings = html.find("<details").unwrap();
    let table = html.find("<table").unwrap();

    assert!(settings < table);
    assert!(!html.contains("<details open"));
}
