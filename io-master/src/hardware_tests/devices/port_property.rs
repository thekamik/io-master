//! The properties of a serial port that can be shown, one per
//! column of the table of ports.
//!
//! A scan knows a lot about every port (`devices.rs`). The user
//! chooses which of it the table shows; that choice is stored by
//! the keys of [`PortProperty`] (`scan_settings.rs`), which never
//! change, so a stored choice stays valid when titles are reworded.

use super::serial_ports::SerialPortInfo;

/// Shown for a property that is not known.
pub const UNKNOWN: &str = "-";

/// One property of a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortProperty {
    /// The device file, `/dev/ttyACM0`.
    Path,
    /// The kernel name, `ttyACM0`.
    Name,
    /// The kind of port, `ttyACM`.
    Type,
    /// What the port belongs to, in words (see [`description`]).
    Device,
    /// The serial number of the device.
    SerialNumber,
    /// What the current user may do with the port.
    Access,
    /// USB vendor and product ids, `2341:1002`.
    UsbIds,
    /// The manufacturer, as the USB device names itself.
    Manufacturer,
    /// The product, as the USB device names itself.
    Product,
    /// Number of the USB interface the port belongs to.
    UsbInterface,
    /// Where the USB device is plugged in, `3-1`.
    UsbLocation,
    /// Speed of the USB connection.
    UsbSpeed,
    /// The kernel driver, `cdc_acm`.
    Driver,
    /// The bus of the hardware, `usb`.
    Bus,
    /// Owner and group of the device file, `root:dialout`.
    Owner,
    /// Permissions of the device file, `0660`.
    Permissions,
    /// The stable names below `/dev/serial/`.
    Links,
    /// The device number, `166:0`.
    DeviceNumber,
    /// The place of the port in the device tree of the kernel.
    SysfsPath,
}

impl PortProperty {
    /// Every property, in the order of the columns.
    pub const ALL: [PortProperty; 19] = [
        PortProperty::Path,
        PortProperty::Name,
        PortProperty::Type,
        PortProperty::Device,
        PortProperty::SerialNumber,
        PortProperty::Access,
        PortProperty::UsbIds,
        PortProperty::Manufacturer,
        PortProperty::Product,
        PortProperty::UsbInterface,
        PortProperty::UsbLocation,
        PortProperty::UsbSpeed,
        PortProperty::Driver,
        PortProperty::Bus,
        PortProperty::Owner,
        PortProperty::Permissions,
        PortProperty::Links,
        PortProperty::DeviceNumber,
        PortProperty::SysfsPath,
    ];

    /// The columns shown until the user chooses others: what is
    /// needed to pick a port and use it.
    pub const DEFAULT: [PortProperty; 5] = [
        PortProperty::Path,
        PortProperty::Type,
        PortProperty::Device,
        PortProperty::SerialNumber,
        PortProperty::Access,
    ];

    /// The key under which the choice is stored. Never changed
    /// once released.
    pub fn key(self) -> &'static str {
        match self {
            PortProperty::Path => "path",
            PortProperty::Name => "name",
            PortProperty::Type => "type",
            PortProperty::Device => "device",
            PortProperty::SerialNumber => "serial_number",
            PortProperty::Access => "access",
            PortProperty::UsbIds => "usb_ids",
            PortProperty::Manufacturer => "manufacturer",
            PortProperty::Product => "product",
            PortProperty::UsbInterface => "usb_interface",
            PortProperty::UsbLocation => "usb_location",
            PortProperty::UsbSpeed => "usb_speed",
            PortProperty::Driver => "driver",
            PortProperty::Bus => "bus",
            PortProperty::Owner => "owner",
            PortProperty::Permissions => "permissions",
            PortProperty::Links => "links",
            PortProperty::DeviceNumber => "device_number",
            PortProperty::SysfsPath => "sysfs_path",
        }
    }

    /// The property with the given key.
    pub fn from_key(key: &str) -> Option<PortProperty> {
        PortProperty::ALL.into_iter().find(|property| property.key() == key)
    }

    /// The title of the column.
    pub fn title(self) -> &'static str {
        match self {
            PortProperty::Path => "Port",
            PortProperty::Name => "Name",
            PortProperty::Type => "Type",
            PortProperty::Device => "Device",
            PortProperty::SerialNumber => "Serial number",
            PortProperty::Access => "Your access",
            PortProperty::UsbIds => "USB ids",
            PortProperty::Manufacturer => "Manufacturer",
            PortProperty::Product => "Product",
            PortProperty::UsbInterface => "USB interface",
            PortProperty::UsbLocation => "USB location",
            PortProperty::UsbSpeed => "USB speed",
            PortProperty::Driver => "Driver",
            PortProperty::Bus => "Bus",
            PortProperty::Owner => "Owner",
            PortProperty::Permissions => "Permissions",
            PortProperty::Links => "Stable names",
            PortProperty::DeviceNumber => "Device number",
            PortProperty::SysfsPath => "Kernel path",
        }
    }

    /// The value of the property for a port, as text; "-" when the
    /// system does not know it.
    pub fn value(self, port: &SerialPortInfo) -> String {
        let usb = port.usb.as_ref();
        let known = |text: Option<&str>| text.unwrap_or(UNKNOWN).to_owned();

        match self {
            PortProperty::Path => port.path.clone(),
            PortProperty::Name => port.name.clone(),
            PortProperty::Type => port.port_type.to_string(),
            PortProperty::Device => description(port),
            PortProperty::SerialNumber => known(serial_number(port)),
            PortProperty::Access => access(port).to_owned(),
            PortProperty::UsbIds => match usb.and_then(|usb| usb.vendor_id.as_deref().zip(usb.product_id.as_deref())) {
                Some((vendor, product)) => format!("{vendor}:{product}"),
                None => UNKNOWN.to_owned(),
            },
            PortProperty::Manufacturer => known(usb.and_then(|usb| usb.manufacturer.as_deref())),
            PortProperty::Product => known(usb.and_then(|usb| usb.product.as_deref())),
            PortProperty::UsbInterface => known(usb.and_then(|usb| usb.interface_number.as_deref())),
            PortProperty::UsbLocation => known(usb.and_then(|usb| usb.location.as_deref())),
            PortProperty::UsbSpeed => usb
                .and_then(|usb| usb.speed_mbps.as_deref())
                .map_or_else(|| UNKNOWN.to_owned(), |speed| format!("{speed} Mbit/s")),
            PortProperty::Driver => known(port.driver.as_deref()),
            PortProperty::Bus => known(port.bus.as_deref()),
            PortProperty::Owner => match &port.access {
                Some(access) => format!(
                    "{}:{}",
                    access.owner.clone().unwrap_or_else(|| access.owner_uid.to_string()),
                    access.group.clone().unwrap_or_else(|| access.group_gid.to_string())
                ),
                None => UNKNOWN.to_owned(),
            },
            PortProperty::Permissions => known(port.access.as_ref().map(|access| access.mode.as_str())),
            PortProperty::Links => {
                if port.links.is_empty() {
                    UNKNOWN.to_owned()
                } else {
                    port.links.join("\n")
                }
            }
            PortProperty::DeviceNumber => port.device_number.clone(),
            PortProperty::SysfsPath => port.sysfs_path.clone(),
        }
    }
}

/// What the port belongs to, in words. The first of these that is
/// known:
///
/// 1. what a USB device says about itself: manufacturer and
///    product, "Arduino UNO WiFi R4 CMSIS-DAP";
/// 2. what the hardware database of udev says about the chip:
///    vendor and model, for example for a PCI serial port;
/// 3. the name of the driver, "serial";
/// 4. "-".
pub fn description(port: &SerialPortInfo) -> String {
    let join = |words: [Option<&str>; 2]| words.into_iter().flatten().collect::<Vec<_>>().join(" ");

    if let Some(usb) = &port.usb {
        let words = join([usb.manufacturer.as_deref(), usb.product.as_deref()]);

        if !words.is_empty() {
            return words;
        }
    }

    let from_database = join([
        port.udev.get("ID_VENDOR_FROM_DATABASE").map(String::as_str),
        port.udev.get("ID_MODEL_FROM_DATABASE").map(String::as_str),
    ]);

    if !from_database.is_empty() {
        return from_database;
    }

    port.driver.clone().unwrap_or_else(|| UNKNOWN.to_owned())
}

/// The serial number of the device: the one the USB device reports,
/// or, without USB, the one udev knows.
fn serial_number(port: &SerialPortInfo) -> Option<&str> {
    port.usb
        .as_ref()
        .and_then(|usb| usb.serial_number.as_deref())
        .or_else(|| port.udev.get("ID_SERIAL_SHORT").map(String::as_str))
}

/// What the user running the program may do with the port.
fn access(port: &SerialPortInfo) -> &'static str {
    match &port.access {
        None => UNKNOWN,
        Some(access) => match (access.readable, access.writable) {
            (true, true) => "read and write",
            (true, false) => "read only",
            (false, true) => "write only",
            (false, false) => "none",
        },
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `port_property.rs`.

    use super::*;

    use std::collections::BTreeMap;

    use crate::hardware_tests::devices::serial_ports::{AccessInfo, PortType, UsbInfo};

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
}
