//! What an installation is: its identifier, made at the first start,
//! and the rules of the name of its one user.
//!
//! The record itself is kept in the system database
//! (`db/system/`). Whether this is the first start, and the
//! installing, are in `installation/lifecycle.rs`.

use std::fmt;

/// The longest name of a user, in characters.
pub const USER_NAME_MAX: usize = 64;

/// Length of the identifier of an installation in bytes.
pub const ID_LENGTH: usize = 16;

/// The identifier of an installation: random, created at the first
/// start, and kept in the system database.
pub type InstallationId = [u8; ID_LENGTH];

/// What the first start made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub id: InstallationId,
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

#[cfg(test)]
mod tests {
    //! Unit tests of `model/installation.rs`.

    use super::*;

    #[test]
    fn user_names_are_checked() {
        for name in ["anna", "Jan Kowalski", "ewaluator-2"] {
            assert!(check_user_name(name).is_ok(), "{name}");
        }

        let too_long = "n".repeat(USER_NAME_MAX + 1);

        for name in ["", " anna", "anna ", "an\nna", too_long.as_str()] {
            assert!(check_user_name(name).is_err(), "{name:?}");
        }
    }
}
