//! Protection of the web interface.
//!
//! The interface listens only on 127.0.0.1, so no other computer
//! can reach it. Two attacks still reach a server on localhost
//! *through the browser of the evaluator*, and both are stopped
//! here:
//!
//! - **DNS rebinding.** A malicious website makes its own domain
//!   name point to 127.0.0.1 and then reads the answers of this
//!   server as if they were its own. Its requests still carry its
//!   domain in the `Host` header, so [`check_host`] accepts only
//!   `127.0.0.1:PORT` and `localhost:PORT`.
//! - **Cross-site requests (CSRF).** A malicious website makes the
//!   browser send a form to this server. The session cookie is
//!   `SameSite=Strict` and is not sent with such requests; in
//!   addition [`check_origin`] refuses every request that changes
//!   something unless it comes from a page of this server.
//!
//! [`security_headers`] adds the headers that tell the browser to
//! load nothing from elsewhere, to let no other page frame this
//! one, and to send no addresses of this server to other sites.

use axum::extract::{Request, State};
use axum::http::header::{
    CONTENT_SECURITY_POLICY, HOST, ORIGIN, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS,
};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::web_ui::AppState;

/// The only scripts, styles and images a page may use are those
/// of this server; no plugins; no framing; forms only to this
/// server.
const CONTENT_SECURITY_POLICY_VALUE: &str = "default-src 'self'; object-src 'none'; \
     base-uri 'none'; frame-ancestors 'none'; form-action 'self'";

/// The names under which the interface is reached in a browser.
fn allowed_hosts(port: u16) -> [String; 2] {
    [format!("127.0.0.1:{port}"), format!("localhost:{port}")]
}

/// Refuses a request whose `Host` header is not this server.
pub async fn check_host(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let host = request.headers().get(HOST).and_then(|value| value.to_str().ok());

    let allowed = allowed_hosts(state.port);

    if host.is_some_and(|host| allowed.iter().any(|name| name == host)) {
        return next.run(request).await;
    }

    // 421 Misdirected Request: this server does not answer for
    // that name.
    (StatusCode::MISDIRECTED_REQUEST, "This server answers only on 127.0.0.1.").into_response()
}

/// Refuses a request that changes something (anything other than
/// GET and HEAD) unless its `Origin` is a page of this server.
pub async fn check_origin(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if matches!(*request.method(), Method::GET | Method::HEAD) {
        return next.run(request).await;
    }

    let origin = request.headers().get(ORIGIN).and_then(|value| value.to_str().ok());

    let allowed = allowed_hosts(state.port).map(|host| format!("http://{host}"));

    // Browsers send Origin with every form submission; a request
    // without one did not come from a page of this server.
    if origin.is_some_and(|origin| allowed.iter().any(|name| name == origin)) {
        return next.run(request).await;
    }

    (StatusCode::FORBIDDEN, "The request did not come from a page of IO Master.").into_response()
}

/// Adds the protective headers to every response.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CONTENT_SECURITY_POLICY_VALUE),
    );
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));

    response
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "security_tests.rs"]
mod tests;
