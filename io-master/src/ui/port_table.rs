//! The table of serial ports shown by the console.
//!
//! A scan knows a lot about every port (see
//! [`crate::devices::devices::SerialPortInfo`]). The console
//! shows a short summary, one line per port, with what is needed
//! to pick a port and use it:
//!
//! ```text
//! PORT          TYPE    DEVICE                        SERIAL        ACCESS
//! /dev/ttyACM0  ttyACM  Arduino UNO WiFi R4 CMSIS-DAP  34B7DA6503B4  read/write
//! /dev/ttyS0    ttyS    Intel Corporation Sunrise...  -             read/write
//! ```
//!
//! The table is built as text, without printing it, so that it
//! can be tested and printed wherever the console wants.

use crate::devices::devices::SerialPortInfo;

/// Shown for a property that is not known.
const UNKNOWN: &str = "-";

/// The longest description shown in the DEVICE column, in
/// characters. Longer ones are cut and end with "...".
const DEVICE_WIDTH_MAX: usize = 40;

/// The titles of the columns, in order.
const HEADERS: [&str; 5] = ["PORT", "TYPE", "DEVICE", "SERIAL", "ACCESS"];

/// Builds the table of the given ports, with a line end after
/// every line. An empty list gives only the header line.
pub fn format_port_table(ports: &[SerialPortInfo]) -> String {
    // The cells of every row, in the order of HEADERS.
    let rows: Vec<[String; 5]> = ports
        .iter()
        .map(|port| {
            [
                port.path.clone(),
                port.port_type.to_string(),
                shorten(&device_description(port), DEVICE_WIDTH_MAX),
                serial_number(port).unwrap_or(UNKNOWN).to_owned(),
                access_text(port).to_owned(),
            ]
        })
        .collect();

    // Every column is as wide as its widest cell or its title.
    let mut widths = HEADERS.map(|header| header.chars().count());

    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }

    let mut table = format_line(&HEADERS, &widths);

    for row in &rows {
        table.push_str(&format_line(row, &widths));
    }

    table
}

/// Builds one line: the cells padded to the widths of their
/// columns and separated by two spaces. The last cell is not
/// padded, so no line ends in spaces.
fn format_line(cells: &[impl AsRef<str>], widths: &[usize]) -> String {
    let last = cells.len() - 1;
    let mut line = String::new();

    for (index, (cell, width)) in cells.iter().zip(widths).enumerate() {
        let cell = cell.as_ref();

        if index == last {
            line.push_str(cell);
        } else {
            line.push_str(&format!("{cell:<width$}  "));
        }
    }

    line.push('\n');
    line
}

/// What the port belongs to, in words.
///
/// The first of these that is known:
///
/// 1. what a USB device says about itself: manufacturer and
///    product, for example "Arduino UNO WiFi R4 CMSIS-DAP";
/// 2. what the hardware database of udev says about the chip:
///    vendor and model, for example for a PCI serial port;
/// 3. the name of the driver, for example "serial";
/// 4. "-".
fn device_description(port: &SerialPortInfo) -> String {
    if let Some(usb) = &port.usb {
        let words = join_known(&[usb.manufacturer.as_deref(), usb.product.as_deref()]);

        if !words.is_empty() {
            return words;
        }
    }

    let from_database = join_known(&[
        port.udev.get("ID_VENDOR_FROM_DATABASE").map(String::as_str),
        port.udev.get("ID_MODEL_FROM_DATABASE").map(String::as_str),
    ]);

    if !from_database.is_empty() {
        return from_database;
    }

    port.driver.clone().unwrap_or_else(|| UNKNOWN.to_owned())
}

/// Joins the known words with spaces and leaves out the unknown
/// ones.
fn join_known(words: &[Option<&str>]) -> String {
    words.iter().flatten().copied().collect::<Vec<_>>().join(" ")
}

/// The serial number of the device: the one the USB device
/// reports, or, without USB, the one udev knows.
fn serial_number(port: &SerialPortInfo) -> Option<&str> {
    port.usb
        .as_ref()
        .and_then(|usb| usb.serial_number.as_deref())
        .or_else(|| port.udev.get("ID_SERIAL_SHORT").map(String::as_str))
}

/// What the user running the program may do with the port.
fn access_text(port: &SerialPortInfo) -> &'static str {
    match &port.access {
        None => UNKNOWN,
        Some(access) => match (access.readable, access.writable) {
            (true, true) => "read/write",
            (true, false) => "read only",
            (false, true) => "write only",
            (false, false) => "none",
        },
    }
}

/// Cuts a text to at most `max` characters; a cut text ends with
/// "...".
fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }

    let kept: String = text.chars().take(max - 3).collect();

    format!("{}...", kept.trim_end())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "port_table_tests.rs"]
mod tests;
