//! The check of the installation at the start of the program.
//!
//! `main` calls [`verify_installation`] once, before the server
//! starts, and starts the server according to the state it gets
//! back. The check itself never prints and never ends the program:
//! it only finds out the state and returns it (CLAUDE.md, "core
//! logic is separate from presentation").
//!
//! ```text
//! state           how the server starts
//! NotInstalled    with the setup page and a one-time setup code
//! Installed       with the login page, the installation opened
//! Damaged         not at all; main reports the reason
//! ```
//!
//! The location is the usual data directory, and the state is told by
//! the system database alone (`lifecycle::open`). (The keyring of the
//! Linux account planned for both was dropped in step 48 of
//! docs/history.md.)

use std::fmt;
use std::path::PathBuf;

use super::lifecycle::{self, LifecycleError, Opened, Running};

/// The states of the installation that a start can find. Every
/// start ends in exactly one of them.
#[derive(Debug)]
pub enum InstallationState {
    /// Nothing is installed yet: the server starts with the setup
    /// page. `data_dir` is where the installation will be made.
    NotInstalled { data_dir: PathBuf },
    /// The installation is valid and opened: the server starts with
    /// the login page.
    Installed { data_dir: PathBuf, running: Running },
    /// The installation cannot be used: the program must not serve
    /// it. Holds the reason.
    Damaged(Damage),
}

/// Why an installation cannot be used.
#[derive(Debug)]
pub enum Damage {
    /// The location of the installation cannot be found: neither
    /// `XDG_DATA_HOME` nor `HOME` is set in the environment.
    NoLocation,
    /// The installation at `data_dir` exists but cannot be opened:
    /// a damaged system database, or one of another program.
    Unreadable { data_dir: PathBuf, error: LifecycleError },
}

impl fmt::Display for Damage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Damage::NoLocation => {
                formatter.write_str("the location of the installation cannot be found: HOME is not set")
            }
            Damage::Unreadable { data_dir, error } => {
                write!(formatter, "the installation in {} cannot be opened: {error}", data_dir.display())
            }
        }
    }
}

/// Finds out the state of the installation of this Linux account.
pub fn verify_installation() -> InstallationState {
    verify_at(default_data_dir())
}

/// The check for a given location; `None` when no location could
/// be found. Kept apart from [`verify_installation`] so that the
/// tests can use temporary folders instead of the real one.
fn verify_at(data_dir: Option<PathBuf>) -> InstallationState {
    let Some(data_dir) = data_dir else {
        return InstallationState::Damaged(Damage::NoLocation);
    };

    match lifecycle::open(&data_dir) {
        Ok(Opened::FirstRun) => InstallationState::NotInstalled { data_dir },
        Ok(Opened::Installed(running)) => InstallationState::Installed { data_dir, running },
        Err(error) => InstallationState::Damaged(Damage::Unreadable { data_dir, error }),
    }
}

/// The usual data directory: `$XDG_DATA_HOME/io-master`, or
/// `~/.local/share/io-master` when that is not set. `None` when
/// neither variable is set.
pub fn default_data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;

    Some(base.join("io-master"))
}

#[cfg(test)]
mod tests {
    //! Unit tests of `verify.rs`, in temporary folders.

    use super::*;

    use std::fs;

    use crate::test_support::TempDir;

    const PASSWORD: &str = "correct horse battery";

    #[test]
    fn without_a_location_the_installation_is_damaged() {
        assert!(matches!(verify_at(None), InstallationState::Damaged(Damage::NoLocation)));
    }

    #[test]
    fn an_empty_folder_is_not_installed() {
        let directory = TempDir::new();

        let InstallationState::NotInstalled { data_dir } = verify_at(Some(directory.path().to_owned())) else {
            panic!("an empty folder must be NotInstalled");
        };

        assert_eq!(data_dir, directory.path());
    }

    #[test]
    fn an_installation_is_found_and_opened() {
        let directory = TempDir::new();
        lifecycle::install(directory.path(), "anna", PASSWORD).unwrap();

        let InstallationState::Installed { data_dir, running } = verify_at(Some(directory.path().to_owned())) else {
            panic!("the installation was not found");
        };

        assert_eq!(data_dir, directory.path());
        assert!(crate::db::system::read::user(&running.system_db, "anna").unwrap().is_some());
    }

    #[test]
    fn a_system_database_that_cannot_be_read_is_damaged() {
        let directory = TempDir::new();
        fs::write(directory.path().join("system.db"), b"not a database").unwrap();

        let state = verify_at(Some(directory.path().to_owned()));

        let InstallationState::Damaged(damage @ Damage::Unreadable { .. }) = state else {
            panic!("a broken system database must be Damaged");
        };

        assert!(damage.to_string().contains("cannot be opened"));
    }
}
