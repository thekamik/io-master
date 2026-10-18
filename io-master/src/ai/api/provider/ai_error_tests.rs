//! Unit tests of `ai_error.rs`.

use super::*;

use serde_json::json;

fn error_path() -> JsonPath {
    JsonPath::parse("error.message").unwrap()
}

#[test]
fn the_status_decides_the_kind() {
    let body = json!({"error": {"message": "explained"}}).to_string();
    let path = error_path();
    let error = |status| AiError::from_answer(status, body.as_bytes(), Some(&path));

    assert_eq!(error(401), AiError::KeyRejected("explained".to_owned()));
    assert_eq!(error(403), AiError::KeyRejected("explained".to_owned()));
    assert_eq!(error(404), AiError::NotFound("explained".to_owned()));
    assert_eq!(error(429), AiError::LimitReached("explained".to_owned()));
    assert_eq!(error(400), AiError::Rejected(400, "explained".to_owned()));
    assert_eq!(error(529), AiError::ServiceFailed(529, "explained".to_owned()));
    assert!(matches!(error(307), AiError::UnexpectedAnswer(text) if text.contains("redirect")));
}

#[test]
fn without_the_path_the_beginning_of_the_body_explains() {
    let error = AiError::from_answer(500, b"<html>\n  Bad   gateway\n</html>", Some(&error_path()));

    assert_eq!(error, AiError::ServiceFailed(500, "<html> Bad gateway </html>".to_owned()));
    assert_eq!(
        AiError::from_answer(500, b"", None),
        AiError::ServiceFailed(500, "no explanation was given".to_owned())
    );
}

#[test]
fn a_long_explanation_is_shortened() {
    let long = "x".repeat(1000);
    let AiError::Rejected(_, message) = AiError::from_answer(400, long.as_bytes(), None) else {
        panic!("a 400 is a rejected request");
    };

    assert_eq!(message.chars().count(), EXPLANATION_LENGTH);
    assert!(message.ends_with('…'));
}
