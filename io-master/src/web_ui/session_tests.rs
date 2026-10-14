//! Unit tests of `session.rs`.

use super::*;

use axum::http::HeaderValue;

/// An account and a key for a session.
fn login(name: &str) -> (Account, DatabaseKey) {
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();

    (
        Account {
            id: 1,
            name: name.to_owned(),
        },
        key,
    )
}

#[test]
fn a_session_belongs_to_its_account() {
    let sessions = Sessions::default();
    let (account, key) = login("anna");

    let token = sessions.create(account.clone(), key).unwrap();

    assert_eq!(sessions.user(&token), Some(account));
    assert_eq!(sessions.user("not-a-token"), None);

    sessions.remove(&token);
    assert_eq!(sessions.user(&token), None);
}

#[test]
fn tokens_are_long_and_never_repeat() {
    let sessions = Sessions::default();

    let (account, key) = login("anna");
    let first = sessions.create(account, key).unwrap();
    let (account, key) = login("anna");
    let second = sessions.create(account, key).unwrap();

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
