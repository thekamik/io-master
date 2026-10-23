//! Discovery of serial ports.
//!
//! [`scan`] lists the serial ports of the computer together with
//! everything the system knows about them. It can be limited to
//! some kinds of port ([`PortFilter`]), for example only `ttyACM`
//! and `ttyUSB`.
//!
//! Where the information comes from
//! --------------------------------
//! Linux describes every device in plain files, so nothing has to
//! be opened and no library is needed:
//!
//! ```text
//! /sys/class/tty/<name>             one entry per terminal device;
//!                                   a link to its place in
//!                                   /sys/devices
//!     dev                           device number, "166:0"
//!     device -> ...                 the hardware the port belongs
//!                                   to; absent for virtual
//!                                   terminals (consoles, ptys)
//!     type                          only for UART controllers: the
//!                                   kind of chip, 0 for none
//!
//! /run/udev/data/c<major>:<minor>   what udev found out, one entry
//!                                   per line:
//!     E:ID_VENDOR=Arduino           a property
//!     S:serial/by-id/usb-...        a link below /dev
//!
//! /dev/<name>                       the device file: owner, group,
//!                                   permissions
//! ```
//!
//! Reading a port's files never opens the port itself. That
//! matters: opening a serial port can reset the board behind it,
//! as it does for an Arduino.
//!
//! Which entries are serial ports
//! ------------------------------
//! Of the entries of `/sys/class/tty` two kinds are left out,
//! because they are not serial ports at all:
//!
//! - entries without `device`: virtual terminals such as
//!   `console`, `tty`, `tty1` or `ptmx`;
//! - UART controller slots whose `type` is 0: the kernel creates
//!   `ttyS0` to `ttyS3` on most PCs whether or not a chip is
//!   there, and such a slot cannot transmit anything.
//!
//! Everything else is reported, whatever its kind: USB ports
//! (`ttyACM*`, `ttyUSB*`), built-in UARTs (`ttyS*`, `ttyAMA*`,
//! ...) and kinds this module has no name for
//! ([`PortType::Other`]).

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use rustix::fs::Access;

/* ---------------------------------------------------------------
 * Result types
 * ------------------------------------------------------------- */

/// Everything known about one serial port.
///
/// A property that the system does not report is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortInfo {
    /// Kernel name, for example `ttyACM0`.
    pub name: String,
    /// The kind of port, told by its name.
    pub port_type: PortType,
    /// The device file to open, for example `/dev/ttyACM0`.
    pub path: String,
    /// Device number, `"major:minor"`, for example `"166:0"`.
    pub device_number: String,
    /// Place of the port in the device tree of the kernel.
    pub sysfs_path: String,
    /// Kernel driver that serves the port, for example `cdc_acm`
    /// (USB modems and boards such as Arduino), `ftdi_sio`,
    /// `cp210x`, `ch341` (USB adapters) or `serial` (PCI UARTs).
    pub driver: Option<String>,
    /// Bus of the hardware: `usb`, `pci`, `pnp`, `platform`, ...
    pub bus: Option<String>,
    /// Details of the USB device, for ports on USB.
    pub usb: Option<UsbInfo>,
    /// Stable names of the port that udev created below `/dev`,
    /// for example `/dev/serial/by-id/usb-Arduino_...-if01`.
    /// Unlike `ttyACM0`, they do not change when devices are
    /// plugged in another order.
    pub links: Vec<String>,
    /// Who may use the device file; `None` when it could not be
    /// examined.
    pub access: Option<AccessInfo>,
    /// All properties udev knows about the port, by name. Empty
    /// when udev has no record of it.
    pub udev: BTreeMap<String, String>,
}

/// Details of the USB device a port belongs to, as the device
/// describes itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsbInfo {
    /// Vendor id, four hexadecimal digits, for example `"2341"`
    /// (Arduino).
    pub vendor_id: Option<String>,
    /// Product id, four hexadecimal digits.
    pub product_id: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    /// Number of the USB interface the port belongs to, two
    /// hexadecimal digits. A device with several ports (or a port
    /// and a debugger, as on some boards) has one per interface.
    pub interface_number: Option<String>,
    /// Where the device is plugged in, for example `"3-1"`: bus 3,
    /// port 1. Stays the same as long as the cable stays in the
    /// same socket.
    pub location: Option<String>,
    /// Speed of the connection in Mbit/s, for example `"12"`.
    pub speed_mbps: Option<String>,
}

/// Who may use the device file of a port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessInfo {
    pub owner_uid: u32,
    /// Name of the owner; `None` when it is not in `/etc/passwd`.
    pub owner: Option<String>,
    pub group_gid: u32,
    /// Name of the group, often `dialout`; `None` when it is not
    /// in `/etc/group`.
    pub group: Option<String>,
    /// Permission bits in octal, for example `"0660"`.
    pub mode: String,
    /// Whether the user running the program may read the port.
    pub readable: bool,
    /// Whether the user running the program may write the port.
    pub writable: bool,
}

/// Why a scan failed.
#[derive(Debug)]
pub enum ScanError {
    /// The list of terminal devices could not be read. Holds the
    /// directory and the reason.
    DeviceListUnavailable { path: PathBuf, source: io::Error },
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::DeviceListUnavailable { path, source } => {
                write!(formatter, "cannot read {}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for ScanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScanError::DeviceListUnavailable { source, .. } => Some(source),
        }
    }
}

/* ---------------------------------------------------------------
 * Port types
 * ------------------------------------------------------------- */

/// The kind of a serial port.
///
/// Linux names a serial port after the driver that serves it: the
/// name is a fixed part chosen by the driver and a number,
/// `ttyACM0`, `ttyAMA1`, `ttymxc2`. The fixed part (the *base
/// name*) tells the kind of hardware, so it is all that is needed
/// to classify a port.
///
/// The list covers the names used by the serial drivers of the
/// Linux kernel. A port whose base name is not on the list is
/// still reported, as [`PortType::Other`], so no port is ever
/// hidden because this list is incomplete.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortType {
    /// `ttyS`: UARTs compatible with the 8250/16550 chips. The
    /// serial ports of PCs, PCI and PCMCIA serial cards, and the
    /// built-in UARTs of many systems on a chip.
    Uart8250,
    /// `ttyACM`: USB devices of the CDC ACM class. Arduino and
    /// most other microcontroller boards, USB modems.
    UsbAcm,
    /// `ttyUSB`: USB-to-serial adapters with their own driver:
    /// FTDI, Silicon Labs CP210x, WCH CH340/CH341, Prolific
    /// PL2303 and others.
    UsbSerial,
    /// `ttyGS`: this computer itself acting as a USB serial
    /// device ("gadget"), on boards with a USB device port.
    UsbGadget,
    /// `ttyAMA`: ARM PrimeCell PL011 UARTs, for example on the
    /// Raspberry Pi.
    ArmPl011,
    /// `ttyAML`: Amlogic SoCs.
    Amlogic,
    /// `ttymxc`: NXP i.MX SoCs.
    NxpImx,
    /// `ttyLP`: NXP LPUART (i.MX 7ULP/8, Layerscape).
    NxpLpuart,
    /// `ttyO`: TI OMAP SoCs, with the older OMAP driver.
    TiOmap,
    /// `ttySAC`: Samsung SoCs.
    Samsung,
    /// `ttySTM`: STMicroelectronics STM32 SoCs.
    Stm32,
    /// `ttyTHS`: high-speed UARTs of NVIDIA Tegra SoCs.
    NvidiaTegra,
    /// `ttyMSM`: Qualcomm SoCs.
    Qualcomm,
    /// `ttyHS`: high-speed UARTs of Qualcomm SoCs.
    QualcommHighSpeed,
    /// `ttyPS`: Xilinx Zynq and Zynq UltraScale+.
    XilinxPs,
    /// `ttyUL`: Xilinx UART Lite, in FPGA designs.
    XilinxUartLite,
    /// `ttySC`: Renesas SCI/SCIF and NXP SC16IS7xx chips on SPI
    /// or I2C.
    RenesasSci,
    /// `ttySIF`: SiFive RISC-V SoCs.
    SiFive,
    /// `ttyMV`: Marvell Armada SoCs.
    Marvell,
    /// `ttyMAX`: Maxim MAX310x chips on SPI or I2C.
    Max310x,
    /// `ttyLTQ`: Lantiq (Intel) network SoCs.
    Lantiq,
    /// `ttyRPMSG`: a channel to another processor of the same
    /// system (remote processor messaging).
    RemoteProcessor,
    /// `ttyGF`: the virtual UART of the Android emulator.
    Goldfish,
    /// `rfcomm`: a serial connection over Bluetooth.
    BluetoothRfcomm,
    /// Any other base name, as found, for example `ttyXYZ`.
    Other(String),
}

/// The known port types with their base names.
const PORT_TYPES: [(PortType, &str); 24] = [
    (PortType::Uart8250, "ttyS"),
    (PortType::UsbAcm, "ttyACM"),
    (PortType::UsbSerial, "ttyUSB"),
    (PortType::UsbGadget, "ttyGS"),
    (PortType::ArmPl011, "ttyAMA"),
    (PortType::Amlogic, "ttyAML"),
    (PortType::NxpImx, "ttymxc"),
    (PortType::NxpLpuart, "ttyLP"),
    (PortType::TiOmap, "ttyO"),
    (PortType::Samsung, "ttySAC"),
    (PortType::Stm32, "ttySTM"),
    (PortType::NvidiaTegra, "ttyTHS"),
    (PortType::Qualcomm, "ttyMSM"),
    (PortType::QualcommHighSpeed, "ttyHS"),
    (PortType::XilinxPs, "ttyPS"),
    (PortType::XilinxUartLite, "ttyUL"),
    (PortType::RenesasSci, "ttySC"),
    (PortType::SiFive, "ttySIF"),
    (PortType::Marvell, "ttyMV"),
    (PortType::Max310x, "ttyMAX"),
    (PortType::Lantiq, "ttyLTQ"),
    (PortType::RemoteProcessor, "ttyRPMSG"),
    (PortType::Goldfish, "ttyGF"),
    (PortType::BluetoothRfcomm, "rfcomm"),
];

impl PortType {
    /// The known port types, in the order of the list above.
    /// [`PortType::Other`] is not among them.
    pub fn known() -> impl Iterator<Item = PortType> {
        PORT_TYPES.into_iter().map(|(port_type, _)| port_type)
    }

    /// The port type with the given base name, for example
    /// `"ttyACM"`. Letter case matters, as it does in Linux:
    /// `ttymxc` and `ttyMXC` are different names.
    pub fn from_base_name(base_name: &str) -> PortType {
        PORT_TYPES
            .iter()
            .find(|(_, known)| *known == base_name)
            .map_or_else(|| PortType::Other(base_name.to_owned()), |(port_type, _)| port_type.clone())
    }

    /// The type of the port with the given name, for example
    /// `ttyACM0`: the name without the number at its end.
    pub fn of_port(name: &str) -> PortType {
        PortType::from_base_name(base_name(name))
    }

    /// The base name of the type, for example `"ttyACM"`.
    pub fn base_name(&self) -> &str {
        match self {
            PortType::Other(base_name) => base_name,
            known => PORT_TYPES.iter().find(|(port_type, _)| port_type == known).map_or("", |(_, base_name)| base_name),
        }
    }
}

impl fmt::Display for PortType {
    /// Shows the type by its base name, as people know it.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.base_name())
    }
}

/// The name of a port without the number at its end:
/// `ttyACM0` -> `ttyACM`.
fn base_name(name: &str) -> &str {
    name.trim_end_matches(|character: char| character.is_ascii_digit())
}

/* ---------------------------------------------------------------
 * Where to look
 * ------------------------------------------------------------- */

/// The places a scan reads. The system uses [`Sources::system`];
/// tests point them at a prepared directory.
#[derive(Debug, Clone)]
struct Sources {
    /// The kernel's list of terminal devices.
    tty_class: PathBuf,
    /// The database of udev.
    udev_data: PathBuf,
    /// Where the device files are.
    dev: PathBuf,
    /// The user and group lists, for names of owners and groups.
    passwd: PathBuf,
    group: PathBuf,
}

impl Sources {
    fn system() -> Sources {
        Sources {
            tty_class: PathBuf::from("/sys/class/tty"),
            udev_data: PathBuf::from("/run/udev/data"),
            dev: PathBuf::from("/dev"),
            passwd: PathBuf::from("/etc/passwd"),
            group: PathBuf::from("/etc/group"),
        }
    }
}

/* ---------------------------------------------------------------
 * Scan
 * ------------------------------------------------------------- */

/// Which ports a scan reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortFilter {
    /// Every serial port.
    All,
    /// Only ports of these kinds. An empty list reports nothing.
    Types(Vec<PortType>),
}

impl PortFilter {
    /// Tells whether a port of this kind is reported.
    pub fn accepts(&self, port_type: &PortType) -> bool {
        match self {
            PortFilter::All => true,
            PortFilter::Types(types) => types.contains(port_type),
        }
    }
}

/// Lists the serial ports of the computer, ordered by name
/// (`ttyACM2` before `ttyACM10`), limited by `filter`.
///
/// A port that disappears while it is being examined (a cable
/// pulled out at that moment) is left out. No port is opened.
pub fn scan(filter: &PortFilter) -> Result<Vec<SerialPortInfo>, ScanError> {
    scan_with(&Sources::system(), filter)
}

/// [`scan`] on the given places.
fn scan_with(sources: &Sources, filter: &PortFilter) -> Result<Vec<SerialPortInfo>, ScanError> {
    let unavailable = |source| ScanError::DeviceListUnavailable { path: sources.tty_class.clone(), source };

    let entries = fs::read_dir(&sources.tty_class).map_err(unavailable)?;

    // The user and group lists are read once for all ports.
    let users = read_id_names(&sources.passwd);
    let groups = read_id_names(&sources.group);

    let mut ports = Vec::new();

    for entry in entries {
        let entry = entry.map_err(unavailable)?;

        // The kind is told by the name alone, so other kinds are
        // passed over before any of their files is read.
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        let port_type = PortType::of_port(name);

        if !filter.accepts(&port_type) {
            continue;
        }

        if let Some(port) = examine(sources, name, port_type, &users, &groups) {
            ports.push(port);
        }
    }

    ports.sort_by(|left, right| natural_key(&left.name).cmp(&natural_key(&right.name)));

    Ok(ports)
}

/// Examines one entry of `/sys/class/tty`. Returns `None` when
/// it is not a serial port, or when it disappeared meanwhile.
fn examine(
    sources: &Sources,
    name: &str,
    port_type: PortType,
    users: &BTreeMap<u32, String>,
    groups: &BTreeMap<u32, String>,
) -> Option<SerialPortInfo> {
    let entry = sources.tty_class.join(name);
    let entry = entry.as_path();

    // Virtual terminals have no hardware behind them.
    let device = fs::canonicalize(entry.join("device")).ok()?;

    // A UART slot without a chip.
    if read_attribute(entry, "type").as_deref() == Some("0") {
        return None;
    }

    let device_number = read_attribute(entry, "dev")?;
    let sysfs_path = fs::canonicalize(entry).ok()?;
    let path = sources.dev.join(name);

    let (udev, links) = read_udev_record(&sources.udev_data, &device_number, &sources.dev);

    Some(SerialPortInfo {
        path: path.display().to_string(),
        device_number,
        sysfs_path: sysfs_path.display().to_string(),
        driver: find_driver(&device),
        bus: find_bus(&device),
        usb: find_usb(&device),
        links,
        access: examine_access(&path, users, groups),
        udev,
        name: name.to_owned(),
        port_type,
    })
}

/* ---------------------------------------------------------------
 * The device tree
 * ------------------------------------------------------------- */

/// Reads one attribute file of a device, without the line end.
/// `None` when it does not exist or is empty.
fn read_attribute(directory: &Path, name: &str) -> Option<String> {
    let text = fs::read_to_string(directory.join(name)).ok()?;
    let text = text.trim_end_matches('\n');

    (!text.is_empty()).then(|| text.to_owned())
}

/// The name of what a link in a device directory points to, for
/// example `cdc_acm` for `driver -> .../drivers/cdc_acm`.
fn link_name(directory: &Path, link: &str) -> Option<String> {
    let target = fs::read_link(directory.join(link)).ok()?;

    Some(target.file_name()?.to_str()?.to_owned())
}

/// The device itself and all devices above it, nearest first.
///
/// Every directory of `/sys/devices` is a device; its parent
/// directory is the device it is attached to. The walk stops at
/// the first directory that is not a device (has no `uevent`).
fn ancestors(device: &Path) -> impl Iterator<Item = &Path> {
    device.ancestors().take_while(|directory| directory.join("uevent").exists())
}

/// Kernel layers between a port and its hardware that say
/// nothing about the hardware: `serial-base` (the port and
/// controller objects of the serial core) and `usb-serial` (the
/// port objects of USB adapters).
const INTERNAL_SUBSYSTEMS: [&str; 2] = ["serial-base", "usb-serial"];

/// The driver that serves the port: the driver of the nearest
/// device that has one, past the `serial-base` layer.
///
/// For USB adapters the `usb-serial` port object is the one that
/// carries the meaningful driver (`ftdi_sio`, `cp210x`), so only
/// `serial-base` is passed over.
fn find_driver(device: &Path) -> Option<String> {
    ancestors(device)
        .filter(|directory| link_name(directory, "subsystem").as_deref() != Some("serial-base"))
        .find_map(|directory| link_name(directory, "driver"))
}

/// The bus of the hardware: the subsystem of the nearest device
/// that is not one of the [`INTERNAL_SUBSYSTEMS`].
fn find_bus(device: &Path) -> Option<String> {
    ancestors(device)
        .filter_map(|directory| link_name(directory, "subsystem"))
        .find(|subsystem| !INTERNAL_SUBSYSTEMS.contains(&subsystem.as_str()))
}

/// Details of the USB device above the port, if there is one.
///
/// Two levels of the device tree are involved: the USB
/// *interface* the port belongs to (it has `bInterfaceNumber`),
/// and the USB *device* that contains it (it has `idVendor`).
fn find_usb(device: &Path) -> Option<UsbInfo> {
    let usb_device = ancestors(device).find(|directory| directory.join("idVendor").exists())?;

    let interface_number = ancestors(device)
        .take_while(|directory| *directory != usb_device)
        .find_map(|directory| read_attribute(directory, "bInterfaceNumber"));

    Some(UsbInfo {
        vendor_id: read_attribute(usb_device, "idVendor"),
        product_id: read_attribute(usb_device, "idProduct"),
        manufacturer: read_attribute(usb_device, "manufacturer"),
        product: read_attribute(usb_device, "product"),
        serial_number: read_attribute(usb_device, "serial"),
        interface_number,
        // The name of the device directory is its location.
        location: usb_device.file_name().and_then(|name| name.to_str()).map(str::to_owned),
        speed_mbps: read_attribute(usb_device, "speed"),
    })
}

/* ---------------------------------------------------------------
 * udev
 * ------------------------------------------------------------- */

/// Reads the udev record of a character device: its properties
/// and its links below `/dev`.
///
/// A device that udev has no record of yields nothing; that is
/// not an error.
fn read_udev_record(udev_data: &Path, device_number: &str, dev: &Path) -> (BTreeMap<String, String>, Vec<String>) {
    let mut properties = BTreeMap::new();
    let mut links = Vec::new();

    // "c" for a character device.
    let Ok(content) = fs::read(udev_data.join(format!("c{device_number}"))) else {
        return (properties, links);
    };

    for line in String::from_utf8_lossy(&content).lines() {
        if let Some(property) = line.strip_prefix("E:") {
            if let Some((key, value)) = property.split_once('=') {
                properties.insert(key.to_owned(), value.to_owned());
            }
        } else if let Some(link) = line.strip_prefix("S:") {
            // Links are stored relative to /dev.
            links.push(dev.join(link).display().to_string());
        }
        // Other lines (tags, timestamps) are internal to udev.
    }

    links.sort();

    (properties, links)
}

/* ---------------------------------------------------------------
 * Access
 * ------------------------------------------------------------- */

/// Reads a user or group list (`/etc/passwd`, `/etc/group`) into
/// a map from id to name.
///
/// Both files have the form `name:password:id:...`. Users and
/// groups that come from elsewhere (a directory service) are not
/// in the files; their names stay unknown. A missing file gives
/// an empty map.
fn read_id_names(path: &Path) -> BTreeMap<u32, String> {
    let Ok(content) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };

    content
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            let id = fields.nth(1)?.parse().ok()?;

            Some((id, name.to_owned()))
        })
        .collect()
}

/// Examines the device file of a port: owner, group, permissions,
/// and what the current user may do with it.
fn examine_access(path: &Path, users: &BTreeMap<u32, String>, groups: &BTreeMap<u32, String>) -> Option<AccessInfo> {
    let metadata = fs::metadata(path).ok()?;

    // access() asks the system itself, so group membership and
    // any other access rules are taken into account. The file is
    // not opened.
    let allowed = |mode| rustix::fs::access(path, mode).is_ok();

    Some(AccessInfo {
        owner_uid: metadata.uid(),
        owner: users.get(&metadata.uid()).cloned(),
        group_gid: metadata.gid(),
        group: groups.get(&metadata.gid()).cloned(),
        mode: format!("{:04o}", metadata.mode() & 0o7777),
        readable: allowed(Access::READ_OK),
        writable: allowed(Access::WRITE_OK),
    })
}

/* ---------------------------------------------------------------
 * Order
 * ------------------------------------------------------------- */

/// Sort key that orders names the way people count:
/// `ttyACM2` before `ttyACM10`. The name is split into its text
/// and the number at its end.
fn natural_key(name: &str) -> (&str, u64, &str) {
    let text = base_name(name);
    let digits = &name[text.len()..];

    // A number too large for u64 sorts last; the full name keeps
    // such names in a fixed order.
    (text, digits.parse().unwrap_or(u64::MAX), name)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `serial_ports.rs`.
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
            let interface = self.device("pci0/usb3/3-1/3-1:1.1", "usb", Some("cdc_acm"), &[("bInterfaceNumber", "01")]);

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

        assert!(matches!(scan_with(&sources, &PortFilter::All), Err(ScanError::DeviceListUnavailable { .. })));
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
        assert_eq!(names(&scan_filtered(&system, &only(&[PortType::Other("ttyRP".to_owned())]))), ["ttyRP0"]);
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
}
