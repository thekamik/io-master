//! Why a question to an AI service failed, in kinds the window can tell
//! apart and explain in plain words.
//!
//! The kind comes from what every HTTP service has in common, the status of
//! the answer; the explanation from the answer itself (its `error.message`,
//! else its beginning on one line), so that the user sees what the service
//! said. No error holds the API key: the key travels only in a header, and
//! every text taken from an answer is masked (`client.rs`).

use std::fmt;

use serde_json::Value;

use crate::db::error::DbError;

/// The longest explanation taken from an answer, in characters.
const EXPLANATION_LENGTH: usize = 300;

/// Why a question to an AI service failed.
#[derive(Debug)]
pub enum AiError {
    /// The service needs an API key and the integration has none (or its
    /// address changed since the key was given).
    NoKey,
    /// The address of the integration is not one the program sends to;
    /// holds the reason.
    Address(String),
    /// There is nothing to ask: no message of the user, or a conversation
    /// that ends with an answer of the model.
    NoQuestion,
    /// The question does not fit into the context of the model; it is
    /// refused before it is sent, never shortened in silence.
    TooLong { estimated: u64, room: u64 },
    /// The service does not accept the API key (HTTP 401 or 403).
    KeyRejected(String),
    /// The service has no such endpoint or model (HTTP 404).
    NotFound(String),
    /// Too many requests in a short time (HTTP 429): trying again later
    /// helps.
    LimitReached(String),
    /// The service refused the request (another HTTP 4xx).
    Rejected(u16, String),
    /// The service failed on its side (HTTP 5xx).
    ServiceFailed(u16, String),
    /// The service reported an error in the middle of an answer.
    Reported(String),
    /// The service could not be reached: no network, a certificate that is
    /// not valid, TLS older than 1.3, a connection that broke or did not
    /// answer in time.
    NoConnection(String),
    /// The service answered something the protocol does not describe.
    UnexpectedAnswer(String),
    /// The database of the user failed while the question was prepared or
    /// written into the journal.
    Db(DbError),
}

impl fmt::Display for AiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::NoKey => formatter.write_str("the integration needs an API key and has none; enter one"),
            AiError::Address(reason) => write!(formatter, "the address of the integration: {reason}"),
            AiError::NoQuestion => formatter.write_str("there is nothing to ask: the last message is not the user's"),
            AiError::TooLong { estimated, room } => write!(
                formatter,
                "the question has about {estimated} tokens and the model takes {room} besides its answer"
            ),
            AiError::KeyRejected(message) => write!(formatter, "the AI service does not accept the API key: {message}"),
            AiError::NotFound(message) => write!(formatter, "the AI service has no such endpoint or model: {message}"),
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
            AiError::Db(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AiError {}

impl From<DbError> for AiError {
    fn from(error: DbError) -> AiError {
        AiError::Db(error)
    }
}

/// The error of an answer that is no success: its HTTP status decides the
/// kind, its body gives the explanation.
///
/// Called by: `client.rs` (an answer that is no success).
/// Calls: [`explanation`].
pub(in crate::ai) fn from_status(status: u16, body: &[u8]) -> AiError {
    let message = explanation(body);

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

/// What the service said: `error.message` of a JSON body (both protocols
/// put it there), else the beginning of the body on one line.
///
/// Called by: [`from_status`].
/// Calls: [`shorten`].
fn explanation(body: &[u8]) -> String {
    if let Ok(value) = serde_json::from_slice::<Value>(body)
        && let Some(message) = value.pointer("/error/message").and_then(Value::as_str)
    {
        return shorten(message);
    }

    let text = String::from_utf8_lossy(body);
    let line = text.split_whitespace().collect::<Vec<&str>>().join(" ");

    if line.is_empty() { "no explanation was given".to_owned() } else { shorten(&line) }
}

/// The text cut to [`EXPLANATION_LENGTH`] characters.
///
/// Called by: [`explanation`]; `openai.rs`, `anthropic.rs` (an error in the
/// middle of an answer).
pub(in crate::ai) fn shorten(text: &str) -> String {
    if text.chars().count() <= EXPLANATION_LENGTH {
        return text.to_owned();
    }

    let mut short: String = text.chars().take(EXPLANATION_LENGTH - 1).collect();

    short.push('…');
    short
}

#[cfg(test)]
mod tests {
    //! Tests of the errors: the kind from the status, the explanation from
    //! the body.

    use super::*;

    #[test]
    fn the_status_decides_the_kind() {
        let body = br#"{"error": {"message": "explained", "type": "x"}}"#;

        assert!(matches!(from_status(401, body), AiError::KeyRejected(text) if text == "explained"));
        assert!(matches!(from_status(404, body), AiError::NotFound(_)));
        assert!(matches!(from_status(429, body), AiError::LimitReached(_)));
        assert!(matches!(from_status(400, body), AiError::Rejected(400, _)));
        assert!(matches!(from_status(529, body), AiError::ServiceFailed(529, _)));
        assert!(matches!(from_status(307, body), AiError::UnexpectedAnswer(text) if text.contains("redirect")));
    }

    #[test]
    fn without_a_message_the_beginning_of_the_body_explains() {
        assert!(matches!(
            from_status(502, b"<html>\n  Bad   gateway\n</html>"),
            AiError::ServiceFailed(502, text) if text == "<html> Bad gateway </html>"
        ));
        assert!(
            matches!(from_status(500, b""), AiError::ServiceFailed(500, text) if text == "no explanation was given")
        );

        let long = shorten(&"x".repeat(1000));

        assert_eq!(long.chars().count(), EXPLANATION_LENGTH);
        assert!(long.ends_with('…'));
    }
}
