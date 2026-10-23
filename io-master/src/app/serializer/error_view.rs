//! How an error of a command reaches the page:
//!
//! ```text
//! {"kind": "wrong_credentials", "message": "wrong name or password", "field": null}
//! ```
//!
//! `kind` lets the page react (for example go back to the login when
//! the session has ended); `message` is the sentence it shows;
//! `field`, for a field of an AI connection that breaks a rule, is
//! the field the page marks (`"url"`, ...).

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::ai::api::completions::AiError;

use super::super::error::AppError;

impl AppError {
    /// The kind of the error as the page knows it.
    fn kind(&self) -> &'static str {
        match self {
            AppError::NotInstalled => "not_installed",
            AppError::AlreadyInstalled => "already_installed",
            AppError::NotLoggedIn => "not_logged_in",
            AppError::WrongCredentials => "wrong_credentials",
            AppError::Invalid(_) | AppError::Connection(_) => "invalid",
            AppError::Ai(AiError::NoConnection(_) | AiError::ServiceFailed(..)) => "ai_unavailable",
            AppError::Ai(_) => "ai_refused",
            AppError::Failed(_) => "failed",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("AppError", 3)?;

        let field = match self {
            AppError::Connection(error) => Some(error.field.name()),
            _ => None,
        };

        error.serialize_field("kind", self.kind())?;
        error.serialize_field("message", &self.to_string())?;
        error.serialize_field("field", &field)?;
        error.end()
    }
}
