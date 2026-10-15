//! Unit tests of `api_key.rs`.

use super::*;

#[test]
fn a_key_is_taken_without_the_spaces_around_it() {
    let key = ApiKey::new("  sk-ant-api03-abc_DEF-123\n".to_owned()).unwrap();

    assert_eq!(key.expose(), "sk-ant-api03-abc_DEF-123");
}

#[test]
fn texts_that_are_no_key_are_refused() {
    assert_eq!(ApiKey::new(String::new()).unwrap_err(), InvalidApiKey::Empty);
    assert_eq!(ApiKey::new(" \n\t".to_owned()).unwrap_err(), InvalidApiKey::Empty);
    assert_eq!(ApiKey::new("sk ant".to_owned()).unwrap_err(), InvalidApiKey::InvalidCharacter);
    assert_eq!(ApiKey::new("sk-ąnt".to_owned()).unwrap_err(), InvalidApiKey::InvalidCharacter);
    assert_eq!(ApiKey::new("sk\u{7}".to_owned()).unwrap_err(), InvalidApiKey::InvalidCharacter);
    assert_eq!(ApiKey::new("k".repeat(MAX_LENGTH + 1)).unwrap_err(), InvalidApiKey::TooLong);
    assert!(ApiKey::new("k".repeat(MAX_LENGTH)).is_ok());
}

#[test]
fn the_key_is_never_shown_by_debug() {
    let key = ApiKey::new("sk-ant-secret".to_owned()).unwrap();
    let shown = format!("{key:?}");

    assert_eq!(shown, "ApiKey(<hidden>)");
    assert!(!shown.contains("secret"));
}

#[test]
fn an_error_does_not_repeat_the_text() {
    let error = ApiKey::new("sk-ant secret".to_owned()).unwrap_err();

    assert!(!error.to_string().contains("secret"));
    assert!(!format!("{error:?}").contains("secret"));
}
