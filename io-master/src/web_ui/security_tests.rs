//! Unit tests of `security.rs`, through the whole router, as an
//! attacker's requests would reach it.

use super::*;

use axum::body::Body;
use axum::http::Request as HttpRequest;
use axum::http::header::CONTENT_TYPE;
use tower::ServiceExt;

use crate::installation::installation::{Installation, Role, State as InstallationState};
use crate::tpm::tpm::TpmAccess;
use crate::web_ui::web_ui::router;

/// An installed workstation with the user "x" on port 8080.
fn state() -> AppState {
    let installation = Installation {
        id: [1; 16],
        role: Role::Workstation,
        users: vec!["x".to_owned()],
    };

    // The routes tested here never reach the TPM.
    AppState::new(
        std::env::temp_dir(),
        InstallationState::Installed(installation),
        None,
        8080,
        TpmAccess { port: 0 },
    )
}

/// Sends a request with the given Host and Origin headers.
async fn send(method: Method, path: &str, host: Option<&str>, origin: Option<&str>) -> Response {
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

    router(state()).oneshot(request).await.unwrap()
}

#[tokio::test]
async fn both_local_names_are_accepted() {
    for host in ["127.0.0.1:8080", "localhost:8080"] {
        let response = send(Method::GET, "/login", Some(host), None).await;

        assert_eq!(response.status(), StatusCode::OK, "{host}");
    }
}

#[tokio::test]
async fn a_foreign_host_is_refused() {
    // What a DNS rebinding attack looks like: the right address,
    // the name of another website.
    for host in [Some("evil.example:8080"), Some("127.0.0.1:9999"), None] {
        let response = send(Method::GET, "/login", host, None).await;

        assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST, "{host:?}");
    }
}

#[tokio::test]
async fn a_form_from_another_website_is_refused() {
    for origin in [Some("https://evil.example"), Some("http://127.0.0.1:9999"), None] {
        let response = send(Method::POST, "/login", Some("127.0.0.1:8080"), origin).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{origin:?}");
    }
}

#[tokio::test]
async fn a_form_from_this_server_is_accepted() {
    let response = send(Method::POST, "/login", Some("127.0.0.1:8080"), Some("http://127.0.0.1:8080")).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn every_answer_carries_the_protective_headers() {
    // Also refusals: the header layer is the outermost one.
    for host in [Some("127.0.0.1:8080"), Some("evil.example")] {
        let response = send(Method::GET, "/login", host, None).await;
        let headers = response.headers();

        assert!(headers[CONTENT_SECURITY_POLICY].to_str().unwrap().contains("frame-ancestors 'none'"));
        assert_eq!(headers[X_FRAME_OPTIONS], "DENY");
        assert_eq!(headers[X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert_eq!(headers[REFERRER_POLICY], "no-referrer");
    }
}
