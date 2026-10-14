//! Unit tests of `port_table.rs`.

use super::*;

use std::collections::BTreeMap;

use crate::devices::devices::{AccessInfo, PortType, UsbInfo};

/// A port with only the properties every port has. The tests
/// add what they need.
fn port(name: &str) -> SerialPortInfo {
    SerialPortInfo {
        name: name.to_owned(),
        port_type: PortType::of_port(name),
        path: format!("/dev/{name}"),
        device_number: "0:0".to_owned(),
        sysfs_path: String::new(),
        driver: None,
        bus: None,
        usb: None,
        links: Vec::new(),
        access: None,
        udev: BTreeMap::new(),
    }
}

/// Access rights of the current user.
fn access(readable: bool, writable: bool) -> Option<AccessInfo> {
    Some(AccessInfo {
        owner_uid: 0,
        owner: Some("root".to_owned()),
        group_gid: 20,
        group: Some("dialout".to_owned()),
        mode: "0660".to_owned(),
        readable,
        writable,
    })
}

/// An Arduino board, as the scan of the development computer
/// reports it.
fn arduino() -> SerialPortInfo {
    SerialPortInfo {
        usb: Some(UsbInfo {
            manufacturer: Some("Arduino".to_owned()),
            product: Some("UNO WiFi R4 CMSIS-DAP".to_owned()),
            serial_number: Some("34B7DA6503B4".to_owned()),
            ..UsbInfo::default()
        }),
        access: access(true, true),
        ..port("ttyACM0")
    }
}

#[test]
fn a_usb_board_is_one_line_under_the_header() {
    let table = format_port_table(&[arduino()]);
    let lines: Vec<&str> = table.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("PORT"));
    assert!(lines[1].starts_with("/dev/ttyACM0  ttyACM"));
    assert!(lines[1].contains("Arduino UNO WiFi R4 CMSIS-DAP"));
    assert!(lines[1].contains("34B7DA6503B4"));
    assert!(lines[1].ends_with("read/write"));
}

#[test]
fn columns_line_up() {
    let mut uart = port("ttyS0");
    uart.driver = Some("serial".to_owned());

    let table = format_port_table(&[arduino(), uart]);
    let lines: Vec<&str> = table.lines().collect();

    // Every column starts at the same place on every line.
    for header in HEADERS {
        let column = lines[0].find(header).unwrap();

        for line in &lines[1..] {
            // A cell starts where its title starts, right after
            // the gap that ends the previous column.
            assert_ne!(line.as_bytes()[column], b' ', "{header} is not aligned in {line:?}");

            if column > 0 {
                assert_eq!(line.as_bytes()[column - 1], b' ');
            }
        }
    }

    // No line ends in padding.
    assert!(lines.iter().all(|line| !line.ends_with(' ')));
}

#[test]
fn the_device_is_described_by_the_best_known_source() {
    // The hardware database, for a port without USB.
    let mut uart = port("ttyS0");
    uart.udev.insert("ID_VENDOR_FROM_DATABASE".to_owned(), "Intel".to_owned());
    uart.udev.insert("ID_MODEL_FROM_DATABASE".to_owned(), "SOL".to_owned());
    uart.driver = Some("serial".to_owned());
    assert_eq!(device_description(&uart), "Intel SOL");

    // The driver, when nothing else is known.
    uart.udev.clear();
    assert_eq!(device_description(&uart), "serial");

    // Nothing at all.
    assert_eq!(device_description(&port("ttyAMA0")), UNKNOWN);

    // A USB device without its own description falls back too.
    let mut adapter = port("ttyUSB0");
    adapter.usb = Some(UsbInfo::default());
    adapter.driver = Some("ftdi_sio".to_owned());
    assert_eq!(device_description(&adapter), "ftdi_sio");
}

#[test]
fn long_descriptions_are_cut() {
    let text = "Sunrise Point-LP Active Management Technology - SOL";

    let short = shorten(text, 20);

    // At most 20 characters; fewer when the cut fell after
    // a space, which is not kept before the dots.
    assert!(short.chars().count() <= 20);
    assert!(short.ends_with("..."));
    assert_eq!(shorten("short", 20), "short");
}

#[test]
fn access_is_told_in_words() {
    let mut board = arduino();

    board.access = access(true, false);
    assert_eq!(access_text(&board), "read only");

    board.access = access(false, false);
    assert_eq!(access_text(&board), "none");

    board.access = None;
    assert_eq!(access_text(&board), UNKNOWN);
}

#[test]
fn unknown_values_are_shown_as_a_dash() {
    let table = format_port_table(&[port("ttyAMA0")]);
    let row = table.lines().nth(1).unwrap();

    assert_eq!(row.split_whitespace().collect::<Vec<_>>(), ["/dev/ttyAMA0", "ttyAMA", "-", "-", "-"]);
}
