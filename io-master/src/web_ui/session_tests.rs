//! Unit tests of `session.rs`.

use super::*;

use axum::http::HeaderValue;

#[test]
fn a_session_belongs_to_its_evaluator() {
    let sessions = Sessions::default();

    let token = sessions.create("anna").unwrap();

    assert_eq!(sessions.user(&token).as_deref(), Some("anna"));
    assert_eq!(sessions.user("not-a-token"), None);

    sessions.remove(&token);
    assert_eq!(sessions.user(&token), None);
}

#[test]
fn tokens_are_long_and_never_repeat() {
    let sessions = Sessions::default();

    let first = sessions.create("anna").unwrap();
    let second = sessions.create("anna").unwrap();

    assert_eq!(first.len(), TOKEN_BYTES * 2);
    assert_ne!(first, second);
}

#[test]
fn the_token_is_found_among_other_cookies() {
    let mut headers = HeaderMap::new();

    headers.append(COOKIE, HeaderValue::from_static("theme=dark; io_master_session=abc123"));
    headers.append(COOKIE, HeaderValue::from_static("other=1"));

    assert_eq!(token_from(&headers), Some("abc123"));
    assert_eq!(token_from(&HeaderMap::new()), None);
}

#[test]
fn the_cookie_is_protected() {
    let cookie = session_cookie("abc");

    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(expired_cookie().contains("Max-Age=0"));
}
