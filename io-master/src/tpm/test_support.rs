//! Help for the tests: temporary directories, and a fresh TPM
//! emulator in its own temporary data directory. The real TPM is
//! never touched.
//!
//! Needs the `swtpm` program (package swtpm).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::emulator::Emulator;
use super::tpm::{Tpm, TpmAccess};

/// A temporary directory, removed when it is dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> TempDir {
        TempDir(new_directory("io-master-test"))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Creates a new, empty temporary directory with a unique name.
fn new_directory(prefix: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let directory = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    fs::create_dir_all(&directory).unwrap();

    directory
}

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
