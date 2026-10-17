//! Unit tests of `text.rs`.

use super::*;

#[test]
fn a_text_keeps_its_length_and_has_no_control_characters() {
    assert!(check_text("name", "Anna", 4, false).is_ok());
    assert_eq!(
        check_text("name", "Annas", 4, false),
        Err(Invalid::new("name", "the name is longer than 4 characters"))
    );
    assert_eq!(check_text("name", "a\tb", 10, false).unwrap_err().reason, "the name holds a control character");
    assert!(check_text("description", "a\nb\tc", 10, true).is_ok());
    assert!(check_text("description", "a\u{7}b", 10, true).is_err());
}
