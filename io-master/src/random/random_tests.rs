//! Unit tests of `random.rs`.
//!
//! Randomness cannot be proven by a test. These tests catch the
//! mistakes that are possible: a buffer that is not filled, or
//! bytes that are obviously not random.

use super::*;

#[test]
fn a_long_buffer_is_filled_completely() {
    // Larger than one getrandom() call is guaranteed to deliver.
    let mut buffer = vec![0u8; 1 << 20];

    fill_random(&mut buffer).unwrap();

    // The last kilobyte was reached, not only the beginning.
    assert!(buffer[buffer.len() - 1024..].iter().any(|&byte| byte != 0));
}

#[test]
fn the_bytes_are_spread_over_all_values() {
    // 2048 random bytes cover most of the 256 possible values
    // (about 255 on average); a broken source, for example one
    // that repeats a pattern, covers few.
    let mut buffer = [0u8; 2048];
    let mut seen = [false; 256];

    fill_random(&mut buffer).unwrap();

    for byte in buffer {
        seen[usize::from(byte)] = true;
    }

    assert!(seen.iter().filter(|&&was_seen| was_seen).count() > 240);
}

#[test]
fn an_empty_buffer_needs_nothing() {
    fill_random(&mut []).unwrap();
}
