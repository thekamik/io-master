//! Help for the tests of the whole program: temporary directories, and
//! a small standard (`standards.rs`).

pub mod db;
pub mod standards;

use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

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
pub fn new_directory(prefix: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let directory = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    // Private, as the program makes the folder of an installation, so
    // that the check of its permissions finds nothing to warn of.
    fs::DirBuilder::new().recursive(true).mode(0o700).create(&directory).unwrap();

    directory
}
