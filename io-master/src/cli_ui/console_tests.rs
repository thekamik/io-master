//! Unit tests of `console.rs`.

use super::*;

use std::collections::BTreeMap;

use std::path::PathBuf;

/// A port as a scan would report it, with only the basic
/// properties.
fn fake_port(name: &str) -> SerialPortInfo {
    SerialPortInfo {
        name: name.to_owned(),
        port_type: PortType::of_port(name),
        path: format!("/dev/{name}"),
        device_number: "0:0".to_owned(),
        sysfs_path: String::new(),
        driver: Some("fake".to_owned()),
        bus: None,
        usb: None,
        links: Vec::new(),
        access: None,
        udev: BTreeMap::new(),
    }
}

/// A scan of a computer with one Arduino and one built-in UART.
/// It filters the way the real scan does.
fn fake_scan(filter: Option<&PortType>) -> Result<Vec<SerialPortInfo>, ScanError> {
    Ok([fake_port("ttyACM0"), fake_port("ttyS0")]
        .into_iter()
        .filter(|port| filter.is_none_or(|wanted| *wanted == port.port_type))
        .collect())
}

/// A scan that cannot read the list of devices.
fn failing_scan(_filter: Option<&PortType>) -> Result<Vec<SerialPortInfo>, ScanError> {
    Err(ScanError::DeviceListUnavailable {
        path: PathBuf::from("/sys/class/tty"),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    })
}

/// Runs the console on the given typed text, with the given scan,
/// and returns what it printed.
fn session_with(typed: &str, scanner: Scanner) -> String {
    let mut output = Vec::new();

    run(&mut typed.as_bytes(), &mut output, scanner).expect("memory never fails");

    String::from_utf8(output).expect("the console prints text")
}

/// Runs the console on the given typed text, with the fake scan.
fn session(typed: &str) -> String {
    session_with(typed, fake_scan)
}

#[test]
fn commands_are_recognized() {
    assert_eq!(parse_command("help\n"), Command::Help);
    assert_eq!(parse_command("  scan  "), Command::Scan(None));
    assert_eq!(parse_command("scan ttyACM"), Command::Scan(Some(PortType::UsbAcm)));
    assert_eq!(parse_command("uart"), Command::Uart);
    assert_eq!(parse_command("ai"), Command::Ai);
    assert_eq!(parse_command("exit"), Command::Exit);
    assert_eq!(parse_command(" \t\n"), Command::Empty);
    assert_eq!(parse_command("HELP"), Command::Unknown("HELP".to_owned()));
}

#[test]
fn help_lists_every_command() {
    let printed = session("help\nexit\n");

    for (name, _) in COMMANDS {
        assert!(printed.contains(name), "help does not mention {name}");
    }
}

#[test]
fn unfinished_commands_say_so() {
    let printed = session("uart\nai\nexit\n");

    assert_eq!(printed.matches(NOT_IMPLEMENTED).count(), 2);
}

#[test]
fn wrong_arguments_are_explained() {
    assert_eq!(
        parse_command("scan ttyACM ttyUSB"),
        Command::Invalid("'scan' takes at most one port type".to_owned())
    );
    assert_eq!(parse_command("exit now"), Command::Invalid("'exit' takes no arguments".to_owned()));
}

#[test]
fn scan_shows_a_table_of_all_ports() {
    let printed = session("scan\nexit\n");

    assert!(printed.contains("PORT"));
    assert!(printed.contains("/dev/ttyACM0"));
    assert!(printed.contains("/dev/ttyS0"));
}

#[test]
fn scan_with_a_type_shows_only_that_type() {
    let printed = session("scan ttyS\nexit\n");

    assert!(printed.contains("/dev/ttyS0"));
    assert!(!printed.contains("/dev/ttyACM0"));
}

#[test]
fn scan_explains_an_empty_result() {
    let printed = session("scan ttyUSB\nexit\n");

    assert!(printed.contains("No serial ports of type ttyUSB found."));
    assert!(!printed.contains("PORT"));
}

#[test]
fn scan_warns_about_an_unknown_type() {
    let printed = session("scan ttyAMC\nexit\n");

    assert!(printed.contains("Note: 'ttyAMC' is not a known port type."));
    assert!(printed.contains("Known types: ttyS, ttyACM, ttyUSB"));
    assert!(printed.contains("No serial ports of type ttyAMC found."));
}

#[test]
fn a_failed_scan_is_reported() {
    let printed = session_with("scan\nexit\n", failing_scan);

    assert!(printed.contains("Cannot scan for serial ports: cannot read /sys/class/tty"));
}

#[test]
fn exit_ends_the_loop() {
    // The line after exit is never read.
    let printed = session("exit\nhelp\n");

    assert!(!printed.contains("Commands:"));
}

#[test]
fn end_of_input_ends_the_loop() {
    let printed = session("");

    assert!(printed.ends_with(&format!("{PROMPT}\n")));
}

#[test]
fn unknown_commands_are_explained() {
    assert!(session("bogus\n").contains("Unknown command: 'bogus'"));
}
