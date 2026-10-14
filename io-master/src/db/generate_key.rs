//! Generating the key that encrypts the database.
//!
//! The key is 32 random bytes from the random number generator of
//! the kernel (see `random/random.rs` for why that source). The
//! bytes are written straight into the memory of the
//! [`DatabaseKey`], so no other copy of them is made.

// TODO: Remove when the database uses this file; until then the
//       compiler sees no user of it.
#![allow(dead_code)]

use std::io;

use crate::db::database_key::DatabaseKey;
use crate::random::random::fill_random;

/// Generates a new key for the database.
///
/// Fails only when the kernel cannot provide random bytes, which
/// on a working Linux system does not happen.
pub fn generate_database_key() -> io::Result<DatabaseKey> {
    DatabaseKey::fill_with(|bytes| fill_random(bytes))
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "generate_key_tests.rs"]
mod tests;
