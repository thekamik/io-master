//! Help for the tests that need a TPM: a fresh emulator in its own
//! temporary data directory. The real TPM is never touched.
//!
//! Needs the `swtpm` program (package swtpm).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::emulator::Emulator;
use super::tpm::{Tpm, TpmAccess};

/// A temporary data directory with its own TPM emulator. Both are
/// removed when it is dropped.
pub struct TestTpm {
    directory: PathBuf,
    emulator: Option<Emulator>,
}

impl TestTpm {
    /// Starts an emulator in a new temporary directory.
    pub fn new() -> TestTpm {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let directory = std::env::temp_dir().join(format!(
            "io-master-tpm-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));

        fs::create_dir_all(&directory).unwrap();

        let emulator = Emulator::start(&directory).expect("swtpm must be installed for this test");

        TestTpm {
            directory,
            emulator: Some(emulator),
        }
    }

    /// The data directory.
    pub fn dir(&self) -> &Path {
        &self.directory
    }

    /// Where the emulator is reached.
    pub fn access(&self) -> TpmAccess {
        TpmAccess {
            port: self.emulator.as_ref().map_or(0, Emulator::port),
        }
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
