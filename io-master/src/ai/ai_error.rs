//! Why a request to the AI service failed, in kinds that a front
//! end can tell apart and explain in plain words.
//!
//! The program knows no provider, so the kind comes from what every
//! HTTP service has in common: the status of the answer. The
//! explanation comes from the answer itself: from where the error
//! path of the connection points (`error.message`, for example), or,
//! when that finds nothing, from the beginning of the answer, so
//! that the evaluator sees what the service said.
//!
//! No error holds the API key: the key travels only in a header, and
//! the client masks it in every text taken from an answer
//! (`ai/client.rs`).

use std::fmt;

use serde_json::Value;

use super::json_path::JsonPath;

/// The longest explanation taken from an answer, in characters.
const EXPLANATION_LENGTH: usize = 300;

/// Why a request to the AI service failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// A header of the connection uses the API key, and none is
    /// stored.
    NoKey,
    /// The service does not accept the API key, or the request
    /// without one (HTTP 401 or 403); holds its explanation.
    KeyRejected(String),
    /// The service has no such endpoint or model (HTTP 404); holds
    /// its explanation.
    NotFound(String),
    /// Too many requests in a short time (HTTP 429); trying again
    /// later helps.
    LimitReached(String),
    /// The service refused the request as it was sent (another HTTP
    /// 4xx); holds the status and the explanation.
    Rejected(u16, String),
    /// The service failed on its side (HTTP 5xx); holds the status
    /// and the explanation.
    ServiceFailed(u16, String),
    /// The service reported an error in the middle of a reply (the
    /// error path found one); holds the explanation.
    Reported(String),
    /// The service could not be reached: no network, no name
    /// resolution, a certificate that is not valid, a connection
    /// that broke or did not answer in time. Holds the explanation.
    NoConnection(String),
    /// The service answered something the connection does not
    /// describe; holds a description.
    UnexpectedAnswer(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::NoKey => {
                formatter.write_str("the connection uses an API key and none is stored; enter one in Settings, tab AI")
            }
            AiError::KeyRejected(message) => write!(formatter, "the AI service does not accept the API key: {message}"),
            AiError::NotFound(message) => {
                write!(formatter, "the AI service has no such endpoint or model: {message}")
            }
            AiError::LimitReached(message) => {
                write!(formatter, "the AI service limits the requests; try again in a moment: {message}")
            }
            AiError::Rejected(status, message) => {
                write!(formatter, "the AI service refused the request (HTTP {status}): {message}")
            }
            AiError::ServiceFailed(status, message) => {
                write!(formatter, "the AI service failed (HTTP {status}): {message}")
            }
            AiError::Reported(message) => write!(formatter, "the AI service reported an error: {message}"),
            AiError::NoConnection(message) => write!(formatter, "the AI service cannot be reached: {message}"),
            AiError::UnexpectedAnswer(message) => write!(formatter, "the AI service answered unexpectedly: {message}"),
        }
    }
}

impl std::error::Error for AiError {}

impl AiError {
    /// The error of an answer that is not a success: its HTTP status
    /// decides the kind, the body gives the explanation.
    pub fn from_answer(status: u16, body: &[u8], error_path: Option<&JsonPath>) -> AiError {
        let message = explanation(body, error_path);

        match status {
            401 | 403 => AiError::KeyRejected(message),
            404 => AiError::NotFound(message),
            429 => AiError::LimitReached(message),
            300..=399 => AiError::UnexpectedAnswer(format!("a redirect (HTTP {status}), which is never followed")),
            400..=499 => AiError::Rejected(status, message),
            500..=599 => AiError::ServiceFailed(status, message),
            _ => AiError::UnexpectedAnswer(format!("HTTP status {status}: {message}")),
        }
    }
}

/// What the service said: the text the error path finds in the body,
/// else the beginning of the body on one line, else that it said
/// nothing.
fn explanation(body: &[u8], error_path: Option<&JsonPath>) -> String {
    if let (Some(path), Ok(value)) = (error_path, serde_json::from_slice::<Value>(body))
        && let Some(message) = path.strings(&value)
    {
        return shorten(&message);
    }

    let text = String::from_utf8_lossy(body);
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if line.is_empty() { "no explanation was given".to_owned() } else { shorten(&line) }
}

/// The text cut to [`EXPLANATION_LENGTH`] characters.
pub fn shorten(text: &str) -> String {
    if text.chars().count() <= EXPLANATION_LENGTH {
        return text.to_owned();
    }

    let mut short: String = text.chars().take(EXPLANATION_LENGTH - 1).collect();
    short.push('…');
    short
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "ai_error_tests.rs"]
mod tests;
