//! Unit tests of `generate_key.rs`.
//!
//! Randomness cannot be proven by a test. These tests catch the
//! mistakes that are possible: a key that is not filled at all,
//! the same key twice, or bytes that are obviously not random.

use super::*;

use crate::keyring::database_key::DATABASE_KEY_LENGTH;

#[test]
fn a_key_is_generated() {
    let key = generate_database_key().unwrap();

    // 32 zero bytes from a working generator has a probability
    // of 2^-256: it means the key was never filled.
    assert_ne!(key.expose(), &[0; DATABASE_KEY_LENGTH]);
}

#[test]
fn every_key_is_different() {
    let first = generate_database_key().unwrap();
    let second = generate_database_key().unwrap();

    assert_ne!(first.expose(), second.expose());
}

#[test]
fn the_bytes_are_spread_over_all_values() {
    // 64 keys give 2048 bytes. Random bytes cover most of the 256
    // possible values (about 255 on average); a broken source,
    // for example one that repeats a pattern, covers few.
    let mut seen = [false; 256];

    for _ in 0..64 {
        let key = generate_database_key().unwrap();

        for &byte in key.expose() {
            seen[usize::from(byte)] = true;
        }
    }

    assert!(seen.iter().filter(|&&was_seen| was_seen).count() > 240);
}

#[test]
fn a_long_buffer_is_filled_completely() {
    // Larger than one getrandom() call is guaranteed to deliver.
    let mut buffer = vec![0u8; 1 << 20];

    fill_random(&mut buffer).unwrap();

    // The last kilobyte was reached, not only the beginning.
    assert!(buffer[buffer.len() - 1024..].iter().any(|&byte| byte != 0));
}
