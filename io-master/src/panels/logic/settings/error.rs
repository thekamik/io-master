//! The error of the settings of the AI: a change refused, an integration
//! the way to the AI refuses (its address, its CA certificate), a failure
//! of the database. The commands of the panel tell its kinds apart for the
//! page.

use std::fmt;

use crate::ai::error::AiError;
use crate::db::error::DbError;

/// Why a change of the settings of the AI was refused or failed.
#[derive(Debug)]
pub enum SettingsError {
    /// The change does not hold together; holds the reason (an integration
    /// without a name, an empty key).
    Refused(String),
    /// The way to the AI refused it (an address, a CA certificate) or
    /// failed.
    Ai(AiError),
    /// A database failed, or refused what was given to it.
    Db(DbError),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::Refused(reason) => formatter.write_str(reason),
            SettingsError::Ai(error) => write!(formatter, "{error}"),
            SettingsError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<AiError> for SettingsError {
    fn from(error: AiError) -> SettingsError {
        SettingsError::Ai(error)
    }
}

impl From<DbError> for SettingsError {
    fn from(error: DbError) -> SettingsError {
        SettingsError::Db(error)
    }
}
