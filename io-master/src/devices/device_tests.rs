//! Unit tests of `devices.rs`.
//!
//! The scan is tested on a small copy of the system's files built
//! in a temporary directory, so no hardware is needed.

use super::*;

use std::os::unix::fs::symlink;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A directory for one test, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> TempDir {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let path = std::env::temp_dir().join(format!(
            "io-master-devices-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        fs::create_dir_all(&path).unwrap();

        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A small copy of the system's files, with the same shape:
/// a device tree, the tty class with links into it, a udev
/// database and device files.
struct FakeSystem {
    root: TempDir,
}

impl FakeSystem {
    fn new() -> FakeSystem {
        let system = FakeSystem { root: TempDir::new() };

        for directory in ["sys/class/tty", "sys/bus", "udev", "dev", "etc"] {
            fs::create_dir_all(system.path(directory)).unwrap();
        }

        system
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.0.join(relative)
    }

    fn sources(&self) -> Sources {
        Sources {
            tty_class: self.path("sys/class/tty"),
            udev_data: self.path("udev"),
            dev: self.path("dev"),
            passwd: self.path("etc/passwd"),
            group: self.path("etc/group"),
        }
    }

    /// Creates a device directory below sys/devices, with
    /// its subsystem, an optional driver and attributes.
    fn device(
        &self,
        relative: &str,
        subsystem: &str,
        driver: Option<&str>,
        attributes: &[(&str, &str)],
    ) -> PathBuf {
        let directory = self.path("sys/devices").join(relative);

        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("uevent"), "").unwrap();

        let bus = self.path("sys/bus").join(subsystem);
        fs::create_dir_all(bus.join("drivers")).unwrap();
        symlink(&bus, directory.join("subsystem")).unwrap();

        if let Some(driver) = driver {
            let driver_directory = bus.join("drivers").join(driver);
            fs::create_dir_all(&driver_directory).unwrap();
            symlink(&driver_directory, directory.join("driver")).unwrap();
        }

        for (name, value) in attributes {
            fs::write(directory.join(name), format!("{value}\n")).unwrap();
        }

        directory
    }

    /// Creates a tty below `parent` and its entry in the tty
    /// class. `parent` is `None` for a virtual terminal.
    fn tty(&self, name: &str, parent: Option<&Path>, number: &str, attributes: &[(&str, &str)]) {
        let directory = match parent {
            Some(parent) => parent.join("tty").join(name),
            None => self.path("sys/devices/virtual/tty").join(name),
        };

        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("uevent"), "").unwrap();
        fs::write(directory.join("dev"), format!("{number}\n")).unwrap();

        for (attribute, value) in attributes {
            fs::write(directory.join(attribute), format!("{value}\n")).unwrap();
        }

        if let Some(parent) = parent {
            symlink(parent, directory.join("device")).unwrap();
        }

        symlink(&directory, self.path("sys/class/tty").join(name)).unwrap();
        fs::write(self.path("dev").join(name), "").unwrap();
    }

    /// Adds an Arduino on USB: device 3-1, interface 1.
    fn add_arduino(&self) {
        self.device(
            "pci0/usb3/3-1",
            "usb",
            Some("usb"),
            &[
                ("idVendor", "2341"),
                ("idProduct", "1002"),
                ("manufacturer", "Arduino"),
                ("product", "UNO WiFi R4 CMSIS-DAP"),
                ("serial", "34B7DA6503B4"),
                ("speed", "12"),
            ],
        );
        let interface = self.device(
            "pci0/usb3/3-1/3-1:1.1",
            "usb",
            Some("cdc_acm"),
            &[("bInterfaceNumber", "01")],
        );

        self.tty("ttyACM0", Some(&interface), "166:0", &[]);

        fs::write(
            self.path("udev/c166:0"),
            "S:serial/by-id/usb-Arduino_UNO-if01\n\
             S:serial/by-path/pci-0-usb-0:1:1.1\n\
             I:3109787856\n\
             E:ID_VENDOR=Arduino\n\
             E:ID_MODEL=UNO_WiFi_R4_CMSIS-DAP\n\
             G:systemd\n",
        )
        .unwrap();
    }
}

fn scan_fake(system: &FakeSystem) -> Vec<SerialPortInfo> {
    scan_filtered(system, &PortFilter::All)
}

fn scan_filtered(system: &FakeSystem, filter: &PortFilter) -> Vec<SerialPortInfo> {
    scan_with(&system.sources(), filter).expect("the fake system can be scanned")
}

/// A filter for the given kinds.
fn only(types: &[PortType]) -> PortFilter {
    PortFilter::Types(types.to_vec())
}

/// The names of the ports, in the order they were returned.
fn names(ports: &[SerialPortInfo]) -> Vec<&str> {
    ports.iter().map(|port| port.name.as_str()).collect()
}

#[test]
fn a_usb_board_is_described_completely() {
    let system = FakeSystem::new();
    system.add_arduino();

    let ports = scan_fake(&system);

    assert_eq!(ports.len(), 1);

    let port = &ports[0];

    assert_eq!(port.name, "ttyACM0");
    assert_eq!(port.port_type, PortType::UsbAcm);
    assert_eq!(port.path, system.path("dev/ttyACM0").display().to_string());
    assert_eq!(port.device_number, "166:0");
    assert_eq!(port.driver.as_deref(), Some("cdc_acm"));
    assert_eq!(port.bus.as_deref(), Some("usb"));

    let usb = port.usb.as_ref().expect("the port is on USB");

    assert_eq!(usb.vendor_id.as_deref(), Some("2341"));
    assert_eq!(usb.product_id.as_deref(), Some("1002"));
    assert_eq!(usb.manufacturer.as_deref(), Some("Arduino"));
    assert_eq!(usb.product.as_deref(), Some("UNO WiFi R4 CMSIS-DAP"));
    assert_eq!(usb.serial_number.as_deref(), Some("34B7DA6503B4"));
    assert_eq!(usb.interface_number.as_deref(), Some("01"));
    assert_eq!(usb.location.as_deref(), Some("3-1"));
    assert_eq!(usb.speed_mbps.as_deref(), Some("12"));

    // Only the properties go into "udev"; the links go into
    // "links", the other lines are left out.
    assert_eq!(port.udev.len(), 2);
    assert_eq!(port.udev["ID_VENDOR"], "Arduino");
    assert_eq!(port.links.len(), 2);
    assert!(port.links[0].ends_with("serial/by-id/usb-Arduino_UNO-if01"));

    // The device file of the fake system is a plain file,
    // which the test may read and write.
    let access = port.access.as_ref().expect("the device file exists");
    assert!(access.readable && access.writable);
    assert_eq!(access.mode.len(), 4);
}

#[test]
fn virtual_terminals_and_empty_uart_slots_are_left_out() {
    let system = FakeSystem::new();

    system.tty("console", None, "5:1", &[]);

    let slot = system.device("platform/serial8250/serial8250:0.1", "serial-base", Some("port"), &[]);
    system.tty("ttyS1", Some(&slot), "4:65", &[("type", "0")]);

    assert!(scan_fake(&system).is_empty());
}

#[test]
fn a_builtin_uart_reports_its_real_bus_and_driver() {
    let system = FakeSystem::new();

    // Newer kernels put two serial-base objects between the
    // port and the PCI controller.
    system.device("pci0/0000:00:16.3", "pci", Some("serial"), &[]);
    system.device("pci0/0000:00:16.3/ctrl", "serial-base", Some("ctrl"), &[]);
    let port = system.device("pci0/0000:00:16.3/ctrl/port", "serial-base", Some("port"), &[]);
    system.tty("ttyS0", Some(&port), "4:64", &[("type", "4")]);

    let ports = scan_fake(&system);

    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].port_type, PortType::Uart8250);
    assert_eq!(ports[0].bus.as_deref(), Some("pci"));
    assert_eq!(ports[0].driver.as_deref(), Some("serial"));
    assert_eq!(ports[0].usb, None);
    // No udev record: no properties and no links, no error.
    assert!(ports[0].udev.is_empty());
    assert!(ports[0].links.is_empty());
}

#[test]
fn a_usb_adapter_reports_the_adapter_driver() {
    let system = FakeSystem::new();

    system.device("pci0/usb1/1-2", "usb", Some("usb"), &[("idVendor", "0403")]);
    system.device("pci0/usb1/1-2/1-2:1.0", "usb", Some("ftdi_sio"), &[("bInterfaceNumber", "00")]);
    let port = system.device("pci0/usb1/1-2/1-2:1.0/ttyUSB0", "usb-serial", Some("ftdi_sio"), &[]);
    system.tty("ttyUSB0", Some(&port), "188:0", &[]);

    let port = &scan_fake(&system)[0];

    assert_eq!(port.driver.as_deref(), Some("ftdi_sio"));
    assert_eq!(port.bus.as_deref(), Some("usb"));
    assert_eq!(port.usb.as_ref().and_then(|usb| usb.vendor_id.as_deref()), Some("0403"));
    assert_eq!(port.usb.as_ref().and_then(|usb| usb.interface_number.as_deref()), Some("00"));
}

#[test]
fn ports_are_ordered_as_people_count() {
    let mut names = vec!["ttyACM10", "ttyS0", "ttyACM2", "ttyACM0"];

    names.sort_by(|left, right| natural_key(left).cmp(&natural_key(right)));

    assert_eq!(names, ["ttyACM0", "ttyACM2", "ttyACM10", "ttyS0"]);
}

#[test]
fn owner_and_group_names_come_from_the_lists() {
    let system = FakeSystem::new();

    fs::write(system.path("etc/group"), "root:x:0:\ndialout:x:20:kamil\nbroken line\n").unwrap();

    let groups = read_id_names(&system.path("etc/group"));

    assert_eq!(groups.get(&20).map(String::as_str), Some("dialout"));
    assert_eq!(groups.len(), 2);
    assert!(read_id_names(&system.path("etc/missing")).is_empty());
}

#[test]
fn a_missing_device_list_is_an_error() {
    let system = FakeSystem::new();
    let mut sources = system.sources();

    sources.tty_class = system.path("sys/class/none");

    assert!(matches!(
        scan_with(&sources, &PortFilter::All),
        Err(ScanError::DeviceListUnavailable { .. })
    ));
}

#[test]
fn the_filter_keeps_one_kind_of_port() {
    let system = FakeSystem::new();
    system.add_arduino();

    let board = system.device("platform/fe201000.serial", "amba", Some("uart-pl011"), &[]);
    system.tty("ttyAMA0", Some(&board), "204:64", &[]);

    // A kind this module has no name for.
    let card = system.device("pci0/0000:02:00.0", "pci", Some("rp2"), &[]);
    system.tty("ttyRP0", Some(&card), "240:0", &[]);

    assert_eq!(names(&scan_fake(&system)), ["ttyACM0", "ttyAMA0", "ttyRP0"]);
    assert_eq!(names(&scan_filtered(&system, &only(&[PortType::UsbAcm]))), ["ttyACM0"]);
    assert_eq!(names(&scan_filtered(&system, &only(&[PortType::ArmPl011]))), ["ttyAMA0"]);
    assert_eq!(
        names(&scan_filtered(&system, &only(&[PortType::Other("ttyRP".to_owned())]))),
        ["ttyRP0"]
    );
    assert!(scan_filtered(&system, &only(&[PortType::UsbSerial])).is_empty());

    // Several kinds at once, and none at all.
    assert_eq!(
        names(&scan_filtered(&system, &only(&[PortType::UsbAcm, PortType::ArmPl011]))),
        ["ttyACM0", "ttyAMA0"]
    );
    assert!(scan_filtered(&system, &only(&[])).is_empty());
}

#[test]
fn port_types_are_told_by_the_name() {
    assert_eq!(PortType::of_port("ttyS0"), PortType::Uart8250);
    assert_eq!(PortType::of_port("ttyACM12"), PortType::UsbAcm);
    assert_eq!(PortType::of_port("ttyUSB3"), PortType::UsbSerial);
    assert_eq!(PortType::of_port("ttymxc1"), PortType::NxpImx);
    assert_eq!(PortType::of_port("rfcomm0"), PortType::BluetoothRfcomm);

    // Base names that begin like another one are not mixed
    // up with it.
    assert_eq!(PortType::of_port("ttySAC2"), PortType::Samsung);
    assert_eq!(PortType::of_port("ttySC0"), PortType::RenesasSci);
    assert_eq!(PortType::of_port("ttySTM1"), PortType::Stm32);

    assert_eq!(PortType::of_port("ttyXYZ7"), PortType::Other("ttyXYZ".to_owned()));
}

#[test]
fn every_known_type_has_its_own_base_name() {
    let known: Vec<PortType> = PortType::known().collect();

    for port_type in &known {
        // The name leads back to the same type.
        assert_eq!(&PortType::from_base_name(port_type.base_name()), port_type);
    }

    let mut base_names: Vec<&str> = known.iter().map(PortType::base_name).collect();
    base_names.sort();
    base_names.dedup();

    assert_eq!(base_names.len(), known.len());
    assert_eq!(PortType::UsbAcm.to_string(), "ttyACM");
}

/// Prints the scan of this computer. Not run by default,
/// because its result depends on the hardware:
///
/// ```text
/// cargo test scan_of_this_computer -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn scan_of_this_computer() {
    println!("{:#?}", scan(&PortFilter::All).expect("the system can be scanned"));
}
