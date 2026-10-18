//! Unit tests of `generate_key.rs`.
//!
//! The source of the bytes is tested in `random/random_tests.rs`;
//! these tests check that a key is filled from it.

use super::*;

use crate::db::encryption::database_key::DATABASE_KEY_LENGTH;

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
