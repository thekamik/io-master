//! Unit tests of `config.rs`.

use super::*;

#[test]
fn a_full_configuration_is_read() {
    let config = parse("role = \"master\"\nsingle_host = true\n").unwrap();

    assert_eq!(
        config,
        Config {
            role: Role::Master,
            single_host: true
        }
    );
}

#[test]
fn optional_settings_have_defaults() {
    let config = parse("role = \"workstation\"").unwrap();

    assert_eq!(config.role, Role::Workstation);
    assert!(!config.single_host);
}

#[test]
fn the_role_is_required() {
    assert_eq!(parse("single_host = true").unwrap_err(), "the setting 'role' is missing");
    assert!(parse("role = \"admin\"").is_err());
    assert!(parse("role = 1").is_err());
}

#[test]
fn wrong_values_are_refused() {
    for text in [
        "role = \"workstation\"\nsingle_host = \"yes\"",
        "role = \"workstation\"\nsingle_host = 1",
    ] {
        assert!(parse(text).is_err(), "{text:?} was accepted");
    }
}

#[test]
fn unknown_settings_are_refused() {
    // The port is a flag of the command line, not a setting.
    assert_eq!(
        parse("role = \"workstation\"\nport = 8080").unwrap_err(),
        "unknown setting 'port'"
    );
    assert_eq!(
        parse("role = \"workstation\"\nsingle_hots = true").unwrap_err(),
        "unknown setting 'single_hots'"
    );
}

#[test]
fn text_that_is_not_toml_is_refused() {
    assert!(parse("role = ").is_err());
}

#[test]
fn a_missing_file_is_reported_as_such() {
    let path = std::env::temp_dir().join("io-master-no-such-config.toml");

    assert!(matches!(load(&path), Err(ConfigError::NotFound(_))));
}
