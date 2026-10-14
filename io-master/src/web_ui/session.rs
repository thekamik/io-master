//! Sessions of logged-in evaluators, and the mock-up login.
//!
//! **Mock-up:** any name is accepted and the password is not
//! checked. The real login, which unlocks the encrypted database
//! with the password, replaces it in step 3 of `docs/todo.md`.
//!
//! A session is a random token in a cookie. The server keeps the
//! tokens in memory, so all sessions end when the program stops.
//!
//! The cookie is `HttpOnly` (scripts of the page cannot read it)
//! and `SameSite=Strict` (the browser does not send it with
//! requests that other websites start). It is not `Secure`,
//! because the interface is served over plain HTTP on 127.0.0.1,
//! which never leaves the computer.

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

use crate::random::random::fill_random;

use super::web_ui::AppState;

/// Name of the session cookie.
pub const COOKIE_NAME: &str = "io_master_session";

/// Random bytes in a token: 256 bits, too many to guess.
const TOKEN_BYTES: usize = 32;

/// The evaluator of the current request, put into the request by
/// [`require_login`] for the pages to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUser(pub String);

/// The open sessions: token -> name of the evaluator.
///
/// Cloning a `Sessions` gives another handle to the same sessions;
/// every request handler gets one.
#[derive(Debug, Clone, Default)]
pub struct Sessions(Arc<Mutex<HashMap<String, String>>>);

impl Sessions {
    /// The table of sessions. A panic in another request while it
    /// held the lock does not leave the table damaged (every
    /// change is a single insert or remove), so the lock is used
    /// as it is.
    fn table(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Opens a session for an evaluator and returns its token.
    pub fn create(&self, user: &str) -> io::Result<String> {
        let mut bytes = [0u8; TOKEN_BYTES];

        fill_random(&mut bytes)?;

        let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

        self.table().insert(token.clone(), user.to_owned());

        Ok(token)
    }

    /// The evaluator of a session, if the token is valid.
    pub fn user(&self, token: &str) -> Option<String> {
        self.table().get(token).cloned()
    }

    /// Ends a session.
    pub fn remove(&self, token: &str) {
        self.table().remove(token);
    }
}

/// The session token sent by the browser, if any.
pub fn token_from(headers: &HeaderMap) -> Option<&str> {
    // A request may carry several Cookie headers, each with
    // several "name=value" pairs separated by ";".
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, token)| token)
}

/// The `Set-Cookie` value that stores a session in the browser.
pub fn session_cookie(token: &str) -> String {
    format!("{COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/")
}

/// The `Set-Cookie` value that removes the session cookie.
pub fn expired_cookie() -> String {
    format!("{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0")
}

/// Lets a request through only with a valid session; others are
/// sent to the login page. The pages find the evaluator as
/// [`CurrentUser`] in the request.
pub async fn require_login(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let user = token_from(request.headers()).and_then(|token| state.sessions.user(token));

    match user {
        Some(user) => {
            request.extensions_mut().insert(CurrentUser(user));
            next.run(request).await
        }
        None => Redirect::to("/login").into_response(),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
