//! Unit tests of `main.rs`.

use super::*;

/// Reads a command line given as words.
fn parse(words: &[&str]) -> Result<Arguments, ArgumentError> {
    let arguments: Vec<String> = words.iter().map(|word| word.to_string()).collect();

    parse_arguments(&arguments)
}

#[test]
fn no_parameters_start_the_local_server_on_8080() {
    assert_eq!(
        parse(&[]),
        Ok(Arguments {
            data_dir: None,
            port: 8080,
            address: None,
            uninstall: false
        })
    );
}

#[test]
fn every_flag_is_read_in_any_order() {
    let arguments = parse(&["--port", "9000", "--data-dir", "/tmp/io", "--address", "10.0.0.5"]).unwrap();

    assert_eq!(arguments.port, 9000);
    assert_eq!(arguments.data_dir, Some(PathBuf::from("/tmp/io")));
    assert_eq!(arguments.address, Some("10.0.0.5".parse().unwrap()));
}

#[test]
fn the_local_address_also_means_the_secure_server() {
    // The flag itself asks for TLS, whatever the address.
    assert_eq!(parse(&["--address", "127.0.0.1"]).unwrap().address, Some("127.0.0.1".parse().unwrap()));
    assert_eq!(parse(&["--address", "::1"]).unwrap().address, Some("::1".parse().unwrap()));
}

#[test]
fn wrong_values_are_refused() {
    for port in ["0", "65536", "http", "-1"] {
        assert!(matches!(parse(&["--port", port]), Err(ArgumentError::InvalidValue(_))), "{port}");
    }

    assert!(matches!(parse(&["--address", "localhost"]), Err(ArgumentError::InvalidValue(_))));
    assert!(matches!(parse(&["--address", "10.0.0"]), Err(ArgumentError::InvalidValue(_))));
}

#[test]
fn mistakes_on_the_command_line_are_refused() {
    assert_eq!(parse(&["--port"]), Err(ArgumentError::MissingValue("--port")));
    assert_eq!(parse(&["--port", "1", "--port", "2"]), Err(ArgumentError::Repeated("--port")));
    assert_eq!(parse(&["--cli"]), Err(ArgumentError::Unknown("--cli".to_owned())));
    assert_eq!(parse(&["9000"]), Err(ArgumentError::Unknown("9000".to_owned())));
}

#[test]
fn uninstall_is_a_flag_without_a_value() {
    let arguments = parse(&["--uninstall", "--data-dir", "/tmp/io"]).unwrap();

    assert!(arguments.uninstall);
    assert_eq!(arguments.data_dir, Some(PathBuf::from("/tmp/io")));
    assert_eq!(parse(&["--uninstall", "--uninstall"]), Err(ArgumentError::Repeated("--uninstall")));
}
