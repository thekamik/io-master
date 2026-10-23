//! Unit tests of `checks.rs`.

/// Halves an even number; an odd one is an error.
fn half(number: u32) -> Result<u32, String> {
    ensure!(number.is_multiple_of(2), format!("{number} is odd"));

    Ok(number / 2)
}

#[test]
fn a_condition_that_holds_lets_the_function_go_on() {
    assert_eq!(half(10), Ok(5));
}

#[test]
fn a_condition_that_fails_returns_the_error() {
    assert_eq!(half(7), Err("7 is odd".to_owned()));
}
