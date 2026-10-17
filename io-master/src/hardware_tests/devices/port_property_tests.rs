//! Unit tests of `port_property.rs`.

use super::*;

use std::collections::BTreeMap;

use crate::hardware_tests::devices::devices::{AccessInfo, PortType, UsbInfo};

/// A port with only the properties every port has.
fn port(name: &str) -> SerialPortInfo {
    SerialPortInfo {
        name: name.to_owned(),
        port_type: PortType::of_port(name),
        path: format!("/dev/{name}"),
        device_number: "166:0".to_owned(),
        sysfs_path: "/sys/devices/x".to_owned(),
        driver: None,
        bus: None,
        usb: None,
        links: Vec::new(),
        access: None,
        udev: BTreeMap::new(),
    }
}

/// An Arduino board, as the scan of the development computer
/// reports it.
fn arduino() -> SerialPortInfo {
    SerialPortInfo {
        driver: Some("cdc_acm".to_owned()),
        bus: Some("usb".to_owned()),
        usb: Some(UsbInfo {
            vendor_id: Some("2341".to_owned()),
            product_id: Some("1002".to_owned()),
            manufacturer: Some("Arduino".to_owned()),
            product: Some("UNO WiFi R4 CMSIS-DAP".to_owned()),
            serial_number: Some("34B7DA6503B4".to_owned()),
            interface_number: Some("01".to_owned()),
            location: Some("3-1".to_owned()),
            speed_mbps: Some("12".to_owned()),
        }),
        links: vec!["/dev/serial/by-id/usb-Arduino-if01".to_owned()],
        access: Some(AccessInfo {
            owner_uid: 0,
            owner: Some("root".to_owned()),
            group_gid: 20,
            group: Some("dialout".to_owned()),
            mode: "0660".to_owned(),
            readable: true,
            writable: true,
        }),
        ..port("ttyACM0")
    }
}

#[test]
fn every_property_has_a_unique_key_that_leads_back_to_it() {
    let mut keys: Vec<&str> = PortProperty::ALL.iter().map(|property| property.key()).collect();

    for property in PortProperty::ALL {
        assert_eq!(PortProperty::from_key(property.key()), Some(property));
    }

    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), PortProperty::ALL.len());
    assert_eq!(PortProperty::from_key("no such key"), None);
}

#[test]
fn the_values_of_a_usb_board() {
    let board = arduino();
    let value = |property: PortProperty| property.value(&board);

    assert_eq!(value(PortProperty::Path), "/dev/ttyACM0");
    assert_eq!(value(PortProperty::Type), "ttyACM");
    assert_eq!(value(PortProperty::Device), "Arduino UNO WiFi R4 CMSIS-DAP");
    assert_eq!(value(PortProperty::SerialNumber), "34B7DA6503B4");
    assert_eq!(value(PortProperty::Access), "read and write");
    assert_eq!(value(PortProperty::UsbIds), "2341:1002");
    assert_eq!(value(PortProperty::UsbSpeed), "12 Mbit/s");
    assert_eq!(value(PortProperty::Owner), "root:dialout");
    assert_eq!(value(PortProperty::Permissions), "0660");
    assert_eq!(value(PortProperty::Links), "/dev/serial/by-id/usb-Arduino-if01");
}

#[test]
fn unknown_values_are_a_dash() {
    let bare = port("ttyAMA0");

    for property in [
        PortProperty::Device,
        PortProperty::SerialNumber,
        PortProperty::Access,
        PortProperty::UsbIds,
        PortProperty::Driver,
        PortProperty::Owner,
        PortProperty::Links,
    ] {
        assert_eq!(property.value(&bare), UNKNOWN, "{property:?}");
    }
}

#[test]
fn the_device_is_described_by_the_best_known_source() {
    let mut uart = port("ttyS0");
    uart.udev.insert("ID_VENDOR_FROM_DATABASE".to_owned(), "Intel".to_owned());
    uart.udev.insert("ID_MODEL_FROM_DATABASE".to_owned(), "SOL".to_owned());
    uart.driver = Some("serial".to_owned());
    assert_eq!(description(&uart), "Intel SOL");

    uart.udev.clear();
    assert_eq!(description(&uart), "serial");

    // A USB device without its own description falls back too.
    let mut adapter = port("ttyUSB0");
    adapter.usb = Some(UsbInfo::default());
    adapter.driver = Some("ftdi_sio".to_owned());
    assert_eq!(description(&adapter), "ftdi_sio");
}

#[test]
fn the_default_columns_are_known_properties() {
    for property in PortProperty::DEFAULT {
        assert!(PortProperty::ALL.contains(&property));
    }
}
