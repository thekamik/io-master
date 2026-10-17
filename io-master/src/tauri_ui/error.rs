//! The errors the commands give back to the page, in kinds the page
//! can tell apart.
//!
//! Every kind carries a sentence for the evaluator; the page shows
//! it as text. No error ever holds a secret (a password, the API
//! key, the key of a database). How an error becomes JSON for the
//! page is in `serializer/`.

use std::fmt;

use crate::accounts::accounts::AccountError;
use crate::ai::api::completions::AiError;
use crate::ai::api::connection::{ConnectionError, Field};
use crate::ai::api::roles::ModelError;
use crate::ai::chat::conversation::SendError;
use crate::db::ai::connection::rows::SaveError;
use crate::db::error::DbError;
use crate::installation::lifecycle::LifecycleError;

/// Why a command failed.
#[derive(Debug)]
pub enum UiError {
    /// Nothing is installed yet; only the setup is possible.
    NotInstalled,
    /// The installation exists already; the setup is no longer
    /// possible.
    AlreadyInstalled,
    /// Nobody is logged in.
    NotLoggedIn,
    /// The name or the password is wrong (one message for both, so
    /// that the page does not tell which names exist).
    WrongCredentials,
    /// What was entered is not accepted; holds the reason.
    Invalid(String),
    /// A field of an AI connection breaks a rule; the page marks
    /// the field.
    Connection(ConnectionError),
    /// The AI service refused or failed.
    Ai(AiError),
    /// The work failed; holds the explanation.
    Failed(String),
}

impl fmt::Display for UiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiError::NotInstalled => formatter.write_str("IO Master is not installed yet"),
            UiError::AlreadyInstalled => formatter.write_str("IO Master is already installed"),
            UiError::NotLoggedIn => formatter.write_str("you are not logged in"),
            UiError::WrongCredentials => formatter.write_str("wrong name or password"),
            UiError::Invalid(reason) => formatter.write_str(reason),
            UiError::Connection(error) => write!(formatter, "{error}"),
            UiError::Ai(error) => write!(formatter, "{error}"),
            UiError::Failed(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for UiError {}

impl From<AccountError> for UiError {
    fn from(error: AccountError) -> UiError {
        match error {
            AccountError::WrongCredentials => UiError::WrongCredentials,
            AccountError::InvalidName(_) | AccountError::NameTaken | AccountError::WeakPassword(_) => {
                UiError::Invalid(error.to_string())
            }
            other => UiError::Failed(other.to_string()),
        }
    }
}

impl From<LifecycleError> for UiError {
    fn from(error: LifecycleError) -> UiError {
        match error {
            LifecycleError::AlreadyInstalled => UiError::AlreadyInstalled,
            LifecycleError::Account(error) => error.into(),
            other => UiError::Failed(other.to_string()),
        }
    }
}

impl From<DbError> for UiError {
    fn from(error: DbError) -> UiError {
        UiError::Failed(error.to_string())
    }
}

impl From<AiError> for UiError {
    fn from(error: AiError) -> UiError {
        UiError::Ai(error)
    }
}

impl From<SendError> for UiError {
    fn from(error: SendError) -> UiError {
        match error {
            SendError::Db(error) => UiError::Failed(format!("the message was not stored: {error}")),
            other => UiError::Invalid(other.to_string()),
        }
    }
}

impl From<ModelError> for UiError {
    fn from(error: ModelError) -> UiError {
        match error {
            ModelError::NoConnection => UiError::Invalid(error.to_string()),
            ModelError::BrokenConnection(_, error) => UiError::Connection(error),
            ModelError::NoKey(_) => UiError::Ai(AiError::NoKey),
            ModelError::Db(error) => UiError::Failed(error.to_string()),
        }
    }
}

impl From<SaveError> for UiError {
    fn from(error: SaveError) -> UiError {
        match error {
            SaveError::Invalid(error) => UiError::Connection(error),
            SaveError::NameTaken => {
                UiError::Connection(ConnectionError { field: Field::Name, reason: error.to_string() })
            }
            SaveError::NoSuchConnection => UiError::Invalid(error.to_string()),
            SaveError::Db(error) => UiError::Failed(error.to_string()),
        }
    }
}
