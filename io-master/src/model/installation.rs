//! What an installation is: its identifier and its role, chosen at
//! the first start, and where its data lives.
//!
//! The record itself is kept in the system database
//! (`db/system/`). Whether this is the first start, and the
//! installing and uninstalling, are in `installation/lifecycle.rs`.

use std::fmt;

/// The longest name of a user, in characters.
pub const USER_NAME_MAX: usize = 64;

/// Length of the identifier of an installation in bytes.
pub const ID_LENGTH: usize = 16;

/// The identifier of an installation: random, created at the first
/// start, and kept both in the system database and in the marker
/// of the TPM, so that the two can be matched.
pub type InstallationId = [u8; ID_LENGTH];

/// The role of the installation (`docs/architecture.md`,
/// section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The computer of an evaluator: every function except the
    /// master panel. Any number of users with equal rights.
    Workstation,
    /// The password reset service: only its panel, and only the
    /// user who installed it.
    Master,
}

impl Role {
    /// The name of the role as stored and shown.
    pub fn name(self) -> &'static str {
        match self {
            Role::Workstation => "workstation",
            Role::Master => "master",
        }
    }

    /// The role with the given name.
    pub fn from_name(name: &str) -> Option<Role> {
        match name {
            "workstation" => Some(Role::Workstation),
            "master" => Some(Role::Master),
            _ => None,
        }
    }

    /// Tells whether users may add further users: on a workstation
    /// they may; a master has only the user who installed it.
    pub fn can_add_users(self) -> bool {
        self == Role::Workstation
    }
}

/// What was chosen at the first start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub id: InstallationId,
    pub role: Role,
}

/// A user name that is not allowed; holds the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidUserName(pub String);

impl fmt::Display for InvalidUserName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid user name: {}", self.0)
    }
}

impl std::error::Error for InvalidUserName {}

/// Checks a user name: 1 to 64 characters, no control characters,
/// no spaces at its ends.
pub fn check_user_name(name: &str) -> Result<(), InvalidUserName> {
    let length = name.chars().count();

    if length == 0 || length > USER_NAME_MAX {
        return Err(InvalidUserName(format!("a name has 1 to {USER_NAME_MAX} characters")));
    }

    if name.trim() != name || name.chars().any(char::is_control) {
        return Err(InvalidUserName(
            "a name may not start or end with a space or contain control characters".to_owned(),
        ));
    }

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "installation_tests.rs"]
mod tests;
