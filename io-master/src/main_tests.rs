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
            port: 8080,
            address: None,
        })
    );
}

#[test]
fn every_flag_is_read_in_any_order() {
    let expected = || Arguments {
        port: 9000,
        address: Some("10.0.0.5".parse().unwrap()),
    };

    assert_eq!(parse(&["--port", "9000", "--address", "10.0.0.5"]), Ok(expected()));
    assert_eq!(parse(&["--address", "10.0.0.5", "--port", "9000"]), Ok(expected()));
}

#[test]
fn the_local_address_also_means_the_secure_server() {
    // The flag itself asks for TLS, whatever the address.
    assert_eq!(parse(&["--address", "127.0.0.1"]).unwrap().address, Some("127.0.0.1".parse().unwrap()));
    assert_eq!(parse(&["--address", "::1"]).unwrap().address, Some("::1".parse().unwrap()));
}

#[test]
fn wrong_values_are_refused() {
    for port in ["0", "65536", "http"] {
        assert!(matches!(parse(&["--port", port]), Err(ArgumentError::InvalidValue(_))), "{port}");
    }

    assert!(matches!(parse(&["--address", "localhost"]), Err(ArgumentError::InvalidValue(_))));
    assert!(matches!(parse(&["--address", "10.0.0"]), Err(ArgumentError::InvalidValue(_))));
}

#[test]
fn mistakes_on_the_command_line_are_refused() {
    assert_eq!(parse(&["--port"]), Err(ArgumentError::MissingValue("--port")));
    assert_eq!(parse(&["--address"]), Err(ArgumentError::MissingValue("--address")));
    assert_eq!(parse(&["--port", "1", "--port", "2"]), Err(ArgumentError::Repeated("--port")));
    assert_eq!(parse(&["--cli"]), Err(ArgumentError::Unknown("--cli".to_owned())));
    assert_eq!(parse(&["9000"]), Err(ArgumentError::Unknown("9000".to_owned())));
}

#[test]
fn a_flag_in_the_place_of_a_value_means_the_value_is_missing() {
    // The value of --address was left out: the next flag is not
    // taken as the address.
    assert_eq!(parse(&["--address", "--port", "8080"]), Err(ArgumentError::MissingValue("--address")));
    assert_eq!(parse(&["--port", "--address", "10.0.0.5"]), Err(ArgumentError::MissingValue("--port")));

    // Also a short or unknown flag, and a negative number.
    assert_eq!(parse(&["--port", "-p"]), Err(ArgumentError::MissingValue("--port")));
    assert_eq!(parse(&["--port", "-1"]), Err(ArgumentError::MissingValue("--port")));

    // An IPv6 address starting with ":" is a value, not a flag.
    assert_eq!(parse(&["--address", "::1"]).unwrap().address, Some("::1".parse().unwrap()));
}
