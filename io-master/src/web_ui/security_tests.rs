//! Unit tests of `security.rs`, through the whole router, as an
//! attacker's requests would reach it.

use super::*;

use axum::body::Body;
use axum::http::Request as HttpRequest;
use axum::http::header::CONTENT_TYPE;
use tower::ServiceExt;

use crate::installation::installation::Role;
use crate::web_ui::test_support::{TestServer, installed};
use crate::web_ui::web_ui::router;

/// An installed workstation with the user "x".
fn server() -> TestServer {
    installed(Role::Workstation, &["x"])
}

/// Sends a request with the given Host and Origin headers.
async fn send(server: &TestServer, method: Method, path: &str, host: Option<&str>, origin: Option<&str>) -> Response {
    let mut builder = HttpRequest::builder().method(method).uri(path);

    if let Some(host) = host {
        builder = builder.header(HOST, host);
    }

    if let Some(origin) = origin {
        builder = builder
            .header(ORIGIN, origin)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded");
    }

    let request = builder.body(Body::from("username=x&password=y")).unwrap();

    router(server.state.clone()).oneshot(request).await.unwrap()
}

#[tokio::test]
async fn both_local_names_are_accepted() {
    let server = server();
    for host in ["127.0.0.1:8080", "localhost:8080"] {
        let response = send(&server, Method::GET, "/login", Some(host), None).await;

        assert_eq!(response.status(), StatusCode::OK, "{host}");
    }
}

#[tokio::test]
async fn a_foreign_host_is_refused() {
    let server = server();
    // What a DNS rebinding attack looks like: the right address,
    // the name of another website.
    for host in [Some("evil.example:8080"), Some("127.0.0.1:9999"), None] {
        let response = send(&server, Method::GET, "/login", host, None).await;

        assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST, "{host:?}");
    }
}

#[tokio::test]
async fn a_form_from_another_website_is_refused() {
    let server = server();

    // "null" is what a sandboxed page or a page with a strict
    // referrer policy sends: not a page of this server.
    for origin in [Some("https://evil.example"), Some("http://127.0.0.1:9999"), Some("null"), None] {
        let response = send(&server, Method::POST, "/login", Some("127.0.0.1:8080"), origin).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{origin:?}");
    }
}

#[tokio::test]
async fn a_form_from_this_server_is_accepted() {
    let server = server();
    let response = send(&server, Method::POST, "/login", Some("127.0.0.1:8080"), Some("http://127.0.0.1:8080")).await;

    // It reaches the login, which refuses the wrong password; the
    // origin check let it through.
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn every_answer_carries_the_protective_headers() {
    let server = server();
    // Also refusals: the header layer is the outermost one.
    for host in [Some("127.0.0.1:8080"), Some("evil.example")] {
        let response = send(&server, Method::GET, "/login", host, None).await;
        let headers = response.headers();

        assert!(headers[CONTENT_SECURITY_POLICY].to_str().unwrap().contains("frame-ancestors 'none'"));
        assert_eq!(headers[X_FRAME_OPTIONS], "DENY");
        assert_eq!(headers[X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(headers[REFERRER_POLICY], "same-origin");
    }
}
