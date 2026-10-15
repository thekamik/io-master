//! Unit tests of `ai_error.rs`.

use super::*;

/// The body Anthropic sends with an error.
fn body(kind: &str, message: &str) -> Vec<u8> {
    serde_json::json!({"type": "error", "error": {"type": kind, "message": message}})
        .to_string()
        .into_bytes()
}

#[test]
fn the_kind_in_the_body_decides() {
    assert_eq!(AiError::from_answer(401, &body("authentication_error", "invalid x-api-key")), AiError::KeyRejected);
    assert_eq!(AiError::from_answer(404, &body("not_found_error", "model: x")), AiError::UnknownModel);
    assert_eq!(AiError::from_answer(429, &body("rate_limit_error", "slow down")), AiError::LimitReached);
    assert_eq!(AiError::from_answer(529, &body("overloaded_error", "busy")), AiError::Overloaded);
    assert_eq!(
        AiError::from_answer(400, &body("invalid_request_error", "Your credit balance is too low")),
        AiError::Rejected("Your credit balance is too low".to_owned())
    );
    assert_eq!(
        AiError::from_answer(402, &body("billing_error", "no card")),
        AiError::Billing("no card".to_owned())
    );
    assert_eq!(
        AiError::from_answer(403, &body("permission_error", "no access")),
        AiError::NotAllowed("no access".to_owned())
    );
    assert_eq!(
        AiError::from_answer(500, &body("api_error", "oops")),
        AiError::ServiceFailed("oops".to_owned())
    );
}

#[test]
fn without_a_body_the_status_decides() {
    assert_eq!(AiError::from_answer(401, b"<html>"), AiError::KeyRejected);
    assert_eq!(AiError::from_answer(429, b""), AiError::LimitReached);
    assert_eq!(AiError::from_answer(529, b""), AiError::Overloaded);
    assert_eq!(AiError::from_answer(502, b""), AiError::ServiceFailed("HTTP status 502".to_owned()));
    assert_eq!(AiError::from_answer(418, b""), AiError::UnexpectedAnswer("HTTP status 418".to_owned()));
}

#[test]
fn an_unknown_kind_is_kept_for_the_explanation() {
    assert_eq!(
        AiError::from_answer(400, &body("new_kind", "text")),
        AiError::UnexpectedAnswer("new_kind: text".to_owned())
    );
}
