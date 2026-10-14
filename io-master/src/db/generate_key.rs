//! Generating the key that encrypts the database.
//!
//! The key is 32 random bytes from the random number generator of
//! the Linux kernel, read with the `getrandom` system call. This
//! is the source cryptographic software on Linux relies on:
//!
//! - The kernel gathers unpredictable events from the whole
//!   machine (interrupt timings, devices, the input of the user,
//!   the random number instructions of the CPU) and turns them
//!   into random bytes with a cryptographic generator (ChaCha20).
//! - `getrandom` waits only once, right after the machine starts,
//!   until the generator has gathered enough; after that it never
//!   waits and never returns weaker bytes. Reading the file
//!   `/dev/urandom` would not wait even at that moment.
//!
//! The bytes are written straight into the memory of the
//! [`DatabaseKey`], so no other copy of them is made.

// TODO: Remove when the database uses this file; until then the
//       compiler sees no user of it.
#![allow(dead_code)]

use std::io;

use rustix::io::Errno;
use rustix::rand::{GetRandomFlags, getrandom};

use crate::keyring::database_key::DatabaseKey;

/// Generates a new key for the database.
///
/// Fails only when the kernel cannot provide random bytes, which
/// on a working Linux system does not happen.
pub fn generate_database_key() -> io::Result<DatabaseKey> {
    DatabaseKey::fill_with(|bytes| fill_random(bytes))
}

/// Fills a buffer with random bytes from the kernel.
fn fill_random(buffer: &mut [u8]) -> io::Result<()> {
    let mut filled = 0;

    // getrandom() may deliver fewer bytes than asked for, or be
    // interrupted by a signal while it waits at start-up; both
    // are handled by asking again for the rest.
    while filled < buffer.len() {
        match getrandom(&mut buffer[filled..], GetRandomFlags::empty()) {
            Ok(0) => return Err(io::Error::other("the kernel returned no random bytes")),
            Ok(count) => filled += count,
            Err(Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        }
    }

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "generate_key_tests.rs"]
mod tests;
