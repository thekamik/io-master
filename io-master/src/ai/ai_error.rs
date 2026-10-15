//! Why a request to the AI service failed, in kinds that a front
//! end can tell apart and explain in plain words.
//!
//! Anthropic answers a failed request with an HTTP status and a
//! JSON body that names the kind of error:
//!
//! ```text
//! {"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}}
//! ```
//!
//! The same body can also arrive in the middle of a streamed reply,
//! as an event named `error`. [`AiError::from_answer`] and
//! [`AiError::from_error_body`] turn both into an [`AiError`].
//!
//! No error ever holds the API key: the key travels only in a
//! header, and Anthropic does not repeat it in its messages.

use std::fmt;

use serde_json::Value;

/// Why a request to the AI service failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// No API key is stored for the evaluator.
    NoKey,
    /// The service does not accept the API key (wrong, revoked).
    KeyRejected,
    /// The key is valid but may not do this; holds the explanation
    /// of the service.
    NotAllowed(String),
    /// The service does not know the model, or the key has no
    /// access to it.
    UnknownModel,
    /// Too many requests or tokens in a short time; trying again
    /// later helps.
    LimitReached,
    /// The account has no credit left, or its billing is not set
    /// up; holds the explanation of the service.
    Billing(String),
    /// The service is overloaded at the moment; trying again later
    /// helps.
    Overloaded,
    /// The service refused the request as it was sent; holds the
    /// explanation of the service. For example a conversation too
    /// long for the model.
    Rejected(String),
    /// The service failed on its side; holds the explanation.
    ServiceFailed(String),
    /// The service could not be reached: no network, no name
    /// resolution, a certificate that is not valid, a connection
    /// that broke or did not answer in time. Holds the
    /// explanation.
    NoConnection(String),
    /// The service answered something this program does not
    /// understand; holds a description.
    UnexpectedAnswer(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::NoKey => formatter.write_str("no API key is stored; enter one in Settings"),
            AiError::KeyRejected => formatter.write_str("Anthropic does not accept the API key"),
            AiError::NotAllowed(message) => write!(formatter, "the API key may not do this: {message}"),
            AiError::UnknownModel => {
                formatter.write_str("the model is not available, or the API key has no access to it")
            }
            AiError::LimitReached => {
                formatter.write_str("the limit of requests of the API key is reached; try again in a moment")
            }
            AiError::Billing(message) => write!(formatter, "the Anthropic account cannot pay for this: {message}"),
            AiError::Overloaded => formatter.write_str("Anthropic is overloaded at the moment; try again later"),
            AiError::Rejected(message) => write!(formatter, "Anthropic refused the request: {message}"),
            AiError::ServiceFailed(message) => write!(formatter, "Anthropic failed: {message}"),
            AiError::NoConnection(message) => write!(formatter, "Anthropic cannot be reached: {message}"),
            AiError::UnexpectedAnswer(message) => write!(formatter, "Anthropic answered unexpectedly: {message}"),
        }
    }
}

impl std::error::Error for AiError {}

impl AiError {
    /// The error of a failed answer: its HTTP status and body. The
    /// kind named in the body decides; the status is used when the
    /// body names none (for example a page of a proxy).
    pub fn from_answer(status: u16, body: &[u8]) -> AiError {
        if let Some(error) = serde_json::from_slice::<Value>(body)
            .ok()
            .and_then(|value| AiError::from_error_body(&value))
        {
            return error;
        }

        match status {
            401 => AiError::KeyRejected,
            402 => AiError::Billing(format!("HTTP status {status}")),
            403 => AiError::NotAllowed(format!("HTTP status {status}")),
            404 => AiError::UnknownModel,
            429 => AiError::LimitReached,
            529 => AiError::Overloaded,
            500..=599 => AiError::ServiceFailed(format!("HTTP status {status}")),
            _ => AiError::UnexpectedAnswer(format!("HTTP status {status}")),
        }
    }

    /// The error named in a JSON error body, or `None` when the
    /// value is not one.
    pub fn from_error_body(value: &Value) -> Option<AiError> {
        let error = value.get("error")?;
        let kind = error.get("type")?.as_str()?;
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("no explanation was given")
            .to_owned();

        Some(match kind {
            "authentication_error" => AiError::KeyRejected,
            "permission_error" => AiError::NotAllowed(message),
            "not_found_error" => AiError::UnknownModel,
            "rate_limit_error" => AiError::LimitReached,
            "billing_error" => AiError::Billing(message),
            "overloaded_error" => AiError::Overloaded,
            "invalid_request_error" | "request_too_large" => AiError::Rejected(message),
            "api_error" => AiError::ServiceFailed(message),
            _ => AiError::UnexpectedAnswer(format!("{kind}: {message}")),
        })
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "ai_error_tests.rs"]
mod tests;
