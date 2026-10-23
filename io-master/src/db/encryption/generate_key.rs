//! Generating the key that encrypts the database.
//!
//! The key is 32 random bytes from the random number generator of
//! the kernel (see `security/random.rs` for why that source). The
//! bytes are written straight into the memory of the
//! [`DatabaseKey`], so no other copy of them is made.

// TODO: Remove when the database uses this file; until then the
//       compiler sees no user of it.
#![allow(dead_code)]

use std::io;

use crate::db::encryption::database_key::DatabaseKey;
use crate::security::random::fill_random;

/// Generates a new key for the database.
///
/// Fails only when the kernel cannot provide random bytes, which
/// on a working Linux system does not happen.
pub fn generate_database_key() -> io::Result<DatabaseKey> {
    DatabaseKey::fill_with(|bytes| fill_random(bytes))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `generate_key.rs`.
    //!
    //! The source of the bytes is tested in `security/random.rs`;
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
}
