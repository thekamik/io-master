//! The error of a command, as the page gets it:
//!
//! ```text
//! {"kind": "wrong_credentials", "message": "the name or the password is wrong"}
//! ```
//!
//! `kind` lets the page react (for example show the login again when the
//! session has ended); `message` is the sentence it shows. No message holds
//! a password, a key or a recovery code.

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::panels::logic::program_start::access::AccountError;
use crate::session::state::SessionBroken;

/// Why a command failed.
#[derive(Debug)]
pub enum ViewError {
    /// The account: a wrong name or password, a weak password, an
    /// installation that exists or does not.
    Account(AccountError),
    /// The session could not be used; the program should be started again.
    Session,
}

impl From<AccountError> for ViewError {
    fn from(error: AccountError) -> ViewError {
        ViewError::Account(error)
    }
}

impl From<SessionBroken> for ViewError {
    fn from(_: SessionBroken) -> ViewError {
        ViewError::Session
    }
}

impl ViewError {
    /// The kind of the error as the page knows it.
    ///
    /// Called by: the serialization below.
    fn kind(&self) -> &'static str {
        match self {
            ViewError::Account(AccountError::NotInstalled) => "not_installed",
            ViewError::Account(AccountError::AlreadyInstalled) => "already_installed",
            ViewError::Account(AccountError::WrongCredentials) => "wrong_credentials",
            ViewError::Account(AccountError::PasswordsDiffer | AccountError::WeakPassword(_) | AccountError::Db(_)) => {
                "invalid"
            }
            ViewError::Account(AccountError::Key(_)) | ViewError::Session => "failed",
        }
    }

    /// The sentence the page shows.
    ///
    /// Called by: the serialization below.
    fn message(&self) -> String {
        match self {
            ViewError::Account(error) => error.to_string(),
            ViewError::Session => "the session of the window failed: start the program again".to_owned(),
        }
    }
}

impl Serialize for ViewError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("ViewError", 2)?;

        error.serialize_field("kind", self.kind())?;
        error.serialize_field("message", &self.message())?;
        error.end()
    }
}
