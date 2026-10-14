//! The configuration of the installation.
//!
//! The configuration says what this installation is. It is
//! written when the program is installed and read at every start;
//! the program refuses to start without it. How the server is
//! reached (port, address) is not part of it: that is chosen with
//! the flags of the command line at every start (`main.rs`).
//!
//! TODO (step 5a of docs/todo.md): this file is temporary. The
//! settings of the installation move into the system database,
//! protected by the secret of the device.
//!
//! The file is `~/.config/io-master/config.toml` (or under
//! `$XDG_CONFIG_HOME`), for example:
//!
//! ```toml
//! # The role of this installation, fixed at installation:
//! # "workstation" (computer of an evaluator) or "master"
//! # (password reset service).
//! role = "workstation"
//!
//! # Master and workstation on the same computer. Not
//! # recommended: see docs/architecture.md, section 3.1.
//! # Optional, false when left out.
//! single_host = false
//! ```
//!
//! An unknown key is an error, so that a typing mistake such as
//! `single_hots` is not silently ignored.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// The role of the installation (`docs/architecture.md`,
/// section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The computer of an evaluator: every function except the
    /// master panel.
    Workstation,
    /// The password reset service: only its panel.
    Master,
}

impl Role {
    /// The name of the role as written in the file.
    pub fn name(self) -> &'static str {
        match self {
            Role::Workstation => "workstation",
            Role::Master => "master",
        }
    }
}

/// The configuration of the installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub role: Role,
    /// Master and workstation on the same computer.
    pub single_host: bool,
}

/// Why the configuration could not be read.
#[derive(Debug)]
pub enum ConfigError {
    /// No place for the file is known: neither `XDG_CONFIG_HOME`
    /// nor `HOME` is set.
    NoLocation,
    /// The file does not exist; the installation is not done.
    NotFound(PathBuf),
    /// The file exists but cannot be read.
    Unreadable { path: PathBuf, source: io::Error },
    /// The file is not a valid configuration; holds the reason.
    Invalid { path: PathBuf, reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NoLocation => {
                formatter.write_str("cannot find the configuration: HOME is not set")
            }
            ConfigError::NotFound(path) => {
                write!(formatter, "no configuration file at {}", path.display())
            }
            ConfigError::Unreadable { path, source } => {
                write!(formatter, "cannot read {}: {}", path.display(), source)
            }
            ConfigError::Invalid { path, reason } => {
                write!(formatter, "{}: {}", path.display(), reason)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// The usual place of the configuration file:
/// `$XDG_CONFIG_HOME/io-master/config.toml`, or
/// `~/.config/io-master/config.toml` when that is not set.
pub fn default_path() -> Result<PathBuf, ConfigError> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or(ConfigError::NoLocation)?;

    Ok(base.join("io-master").join("config.toml"))
}

/// Reads and checks the configuration file.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(ConfigError::NotFound(path.to_owned()));
        }
        Err(source) => {
            return Err(ConfigError::Unreadable {
                path: path.to_owned(),
                source,
            });
        }
    };

    parse(&text).map_err(|reason| ConfigError::Invalid {
        path: path.to_owned(),
        reason,
    })
}

/// Checks the text of a configuration file. The error is the
/// reason, in words.
fn parse(text: &str) -> Result<Config, String> {
    let table: toml::Table = text.parse().map_err(|error: toml::de::Error| {
        // The message of the parser names the line and column.
        error.message().to_owned()
    })?;

    for key in table.keys() {
        if !["role", "single_host"].contains(&key.as_str()) {
            return Err(format!("unknown setting '{key}'"));
        }
    }

    let role = match table.get("role").map(|value| value.as_str()) {
        None => return Err("the setting 'role' is missing".to_owned()),
        Some(Some("workstation")) => Role::Workstation,
        Some(Some("master")) => Role::Master,
        Some(_) => return Err("'role' must be \"workstation\" or \"master\"".to_owned()),
    };

    let single_host = match table.get("single_host") {
        None => false,
        Some(value) => value
            .as_bool()
            .ok_or("'single_host' must be true or false")?,
    };

    Ok(Config { role, single_host })
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
