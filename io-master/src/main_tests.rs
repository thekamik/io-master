//! Unit tests of `main.rs`.

use super::*;

/// Builds the parameter list of a test.
fn arguments(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| text.to_string()).collect()
}

#[test]
fn cli_starts_the_console() {
    assert_eq!(parse_arguments(&arguments(&["--cli"])), Ok(Mode::Console));
}

#[test]
fn a_parameter_is_required() {
    assert_eq!(parse_arguments(&[]), Err(ArgumentError::Missing));
}

#[test]
fn unknown_parameters_are_refused() {
    assert_eq!(
        parse_arguments(&arguments(&["--gui"])),
        Err(ArgumentError::Unknown("--gui".to_owned()))
    );
    assert_eq!(
        parse_arguments(&arguments(&["--cli", "now"])),
        Err(ArgumentError::Unexpected("now".to_owned()))
    );
}
