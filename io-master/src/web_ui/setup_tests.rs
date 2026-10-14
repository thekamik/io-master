//! Unit tests of `setup.rs`: the setup code. The setup page itself
//! is tested through the router in `web_ui_tests.rs`.

use super::*;

#[test]
fn a_code_has_ten_unambiguous_characters() {
    let code = SetupCode::generate().unwrap();

    assert_eq!(code.0.len(), CODE_LENGTH);
    assert!(code.0.bytes().all(|byte| CODE_ALPHABET.contains(&byte)));
    assert_eq!(code.display().len(), CODE_LENGTH + 1);
}

#[test]
fn every_code_is_different() {
    assert_ne!(SetupCode::generate().unwrap(), SetupCode::generate().unwrap());
}

#[test]
fn a_code_is_recognized_as_typed() {
    let code = SetupCode("ABCDEFGHJK".to_owned());

    assert!(code.matches("ABCDE-FGHJK"));
    assert!(code.matches("abcde fghjk"));
    assert!(code.matches("ABCDEFGHJK"));
}

#[test]
fn a_wrong_code_is_refused() {
    let code = SetupCode("ABCDEFGHJK".to_owned());

    for typed in ["", "ABCDE-FGHJ", "ABCDE-FGHJKL", "ABCDE-FGHJL", "BBCDE-FGHJK"] {
        assert!(!code.matches(typed), "{typed:?}");
    }
}
