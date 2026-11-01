//! Random bytes from the kernel.
//!
//! Every secret the program creates (the key of the database,
//! session tokens, ...) comes from here: the random number
//! generator of the Linux kernel, read with the `getrandom`
//! system call. This is the source cryptographic software on
//! Linux relies on:
//!
//! - The kernel gathers unpredictable events from the whole
//!   machine (interrupt timings, devices, the input of the user,
//!   the random number instructions of the CPU) and turns them
//!   into random bytes with a cryptographic generator (ChaCha20).
//! - `getrandom` waits only once, right after the machine starts,
//!   until the generator has gathered enough; after that it never
//!   waits and never returns weaker bytes. Reading the file
//!   `/dev/urandom` would not wait even at that moment.

use std::io;

use rustix::io::Errno;
use rustix::rand::{GetRandomFlags, getrandom};

/// Fills a buffer with random bytes from the kernel.
///
/// Fails only when the kernel cannot provide random bytes, which
/// on a working Linux system does not happen.
pub fn fill_random(buffer: &mut [u8]) -> io::Result<()> {
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

#[cfg(test)]
mod tests {
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
}
