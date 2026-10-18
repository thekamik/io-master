//! Help for the tests: a fresh TPM emulator in its own temporary
//! data directory (`crate::test_support`). The real TPM is
//! never touched.
//!
//! Needs the `swtpm` program (package swtpm).

use std::fs;
use std::path::{Path, PathBuf};

use super::chip::{Tpm, TpmAccess};
use super::emulator::Emulator;
use crate::test_support::new_directory;

/// A temporary data directory with its own TPM emulator. Both are
/// removed when it is dropped.
pub struct TestTpm {
    directory: PathBuf,
    emulator: Option<Emulator>,
}

impl TestTpm {
    /// Starts an emulator in a new temporary directory.
    pub fn new() -> TestTpm {
        let directory = new_directory("io-master-tpm");

        let emulator = Emulator::start(&directory).expect("swtpm must be installed for this test");

        TestTpm { directory, emulator: Some(emulator) }
    }

    /// The data directory.
    pub fn dir(&self) -> &Path {
        &self.directory
    }

    /// Where the emulator is reached.
    pub fn access(&self) -> TpmAccess {
        TpmAccess { port: self.emulator.as_ref().map_or(0, Emulator::port) }
    }

    /// A new connection to the emulator.
    pub fn connect(&self) -> Tpm {
        self.access().connect().expect("the emulator accepts connections")
    }
}

impl Drop for TestTpm {
    fn drop(&mut self) {
        // The emulator first, then its files.
        self.emulator = None;

        let _ = fs::remove_dir_all(&self.directory);
    }
}
