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

use super::super::error::UiError;

impl UiError {
    /// The kind of the error as the page knows it.
    fn kind(&self) -> &'static str {
        match self {
            UiError::NotInstalled => "not_installed",
            UiError::AlreadyInstalled => "already_installed",
            UiError::NotLoggedIn => "not_logged_in",
            UiError::WrongCredentials => "wrong_credentials",
            UiError::Invalid(_) | UiError::Connection(_) => "invalid",
            UiError::Ai(AiError::NoConnection(_) | AiError::ServiceFailed(..)) => "ai_unavailable",
            UiError::Ai(_) => "ai_refused",
            UiError::Failed(_) => "failed",
        }
    }
}

impl Serialize for UiError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("UiError", 3)?;

        let field = match self {
            UiError::Connection(error) => Some(error.field.name()),
            _ => None,
        };

        error.serialize_field("kind", self.kind())?;
        error.serialize_field("message", &self.to_string())?;
        error.serialize_field("field", &field)?;
        error.end()
    }
}
