//! The errors the commands of the views give back to the page, in kinds
//! the page can tell apart.
//!
//! Every kind carries a sentence for the evaluator; the page shows
//! it as text. No error ever holds a secret (a password, the API
//! key, the key of a database). How an error becomes JSON for the
//! page is in `serializer/`.

use std::fmt;

use crate::ai::api::completions::AiError;
use crate::ai::api::connections::ConnectionSaveError;
use crate::ai::api::roles::ModelError;
use crate::db::error::DbError;
use crate::db::oscal::error::OscalError;
use crate::hardware_tests::devices::scan_settings::ScanSettingsError;
use crate::installation::accounts::AccountError;
use crate::installation::lifecycle::LifecycleError;
use crate::model::ai::{ConnectionError, Field};
use crate::model::oscal::catalog::CatalogError;

/// Why a command failed.
#[derive(Debug)]
pub enum AppError {
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

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotInstalled => formatter.write_str("IO Master is not installed yet"),
            AppError::AlreadyInstalled => formatter.write_str("IO Master is already installed"),
            AppError::NotLoggedIn => formatter.write_str("you are not logged in"),
            AppError::WrongCredentials => formatter.write_str("wrong name or password"),
            AppError::Invalid(reason) => formatter.write_str(reason),
            AppError::Connection(error) => write!(formatter, "{error}"),
            AppError::Ai(error) => write!(formatter, "{error}"),
            AppError::Failed(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for AppError {}

impl From<AccountError> for AppError {
    fn from(error: AccountError) -> AppError {
        match error {
            AccountError::WrongCredentials => AppError::WrongCredentials,
            AccountError::InvalidName(_) | AccountError::NameTaken | AccountError::WeakPassword(_) => {
                AppError::Invalid(error.to_string())
            }
            other => AppError::Failed(other.to_string()),
        }
    }
}

impl From<LifecycleError> for AppError {
    fn from(error: LifecycleError) -> AppError {
        match error {
            LifecycleError::AlreadyInstalled => AppError::AlreadyInstalled,
            LifecycleError::Account(error) => error.into(),
            other => AppError::Failed(other.to_string()),
        }
    }
}

impl From<DbError> for AppError {
    fn from(error: DbError) -> AppError {
        AppError::Failed(error.to_string())
    }
}

impl From<AiError> for AppError {
    fn from(error: AiError) -> AppError {
        AppError::Ai(error)
    }
}

impl From<ModelError> for AppError {
    fn from(error: ModelError) -> AppError {
        match error {
            ModelError::NoConnection => AppError::Invalid(error.to_string()),
            ModelError::BrokenConnection(_, error) => AppError::Connection(error),
            ModelError::NoKey(_) => AppError::Ai(AiError::NoKey),
            ModelError::Db(error) => AppError::Failed(error.to_string()),
        }
    }
}

impl From<ConnectionSaveError> for AppError {
    fn from(error: ConnectionSaveError) -> AppError {
        match error {
            ConnectionSaveError::Invalid(error) => AppError::Connection(error),
            ConnectionSaveError::NameTaken => {
                AppError::Connection(ConnectionError { field: Field::Name, reason: error.to_string() })
            }
            ConnectionSaveError::NoSuchConnection => AppError::Invalid(error.to_string()),
            ConnectionSaveError::Db(error) => AppError::Failed(error.to_string()),
        }
    }
}

impl From<OscalError> for AppError {
    fn from(error: OscalError) -> AppError {
        match error {
            OscalError::Db(error) => AppError::Failed(error.to_string()),
            other => AppError::Invalid(other.to_string()),
        }
    }
}

impl From<ScanSettingsError> for AppError {
    fn from(error: ScanSettingsError) -> AppError {
        AppError::Invalid(error.to_string())
    }
}

impl From<CatalogError> for AppError {
    fn from(error: CatalogError) -> AppError {
        AppError::Invalid(error.to_string())
    }
}
