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
//! TODO (step 13 of docs/todo.md): the location of the installation
//! and the secret of the application are read from the keyring of
//! the Linux account, and the states of docs/architecture.md,
//! section 3.2, are told apart here (a lost keyring, an entry
//! without its installation, ...). Until then the location is the
//! usual data directory, and the state is told by the system
//! database alone (`lifecycle::open`).

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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
