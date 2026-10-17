//! Unit tests of `database_key.rs`.

use super::*;

#[test]
fn a_key_is_filled_in_place() {
    let key = DatabaseKey::fill_with(|bytes| {
        bytes.fill(0x5a);
        Ok::<(), ()>(())
    })
    .unwrap();

    assert_eq!(key.expose(), &[0x5a; DATABASE_KEY_LENGTH]);
}

#[test]
fn a_failed_fill_gives_no_key() {
    let result = DatabaseKey::fill_with(|bytes| {
        bytes[0] = 1;
        Err("no randomness")
    });

    assert_eq!(result.unwrap_err(), "no randomness");
}

#[test]
fn a_key_is_never_shown_in_debug_output() {
    let key = DatabaseKey::fill_with(|bytes| {
        bytes.fill(0x5a);
        Ok::<(), ()>(())
    })
    .unwrap();

    assert_eq!(format!("{key:?}"), "DatabaseKey(<hidden>)");
}
