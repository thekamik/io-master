//! Unit tests of `web_ui.rs`: the routes, as a browser sees them.
//!
//! The requests go straight to the router, without a network
//! port. Every request carries the headers a browser would send
//! to `http://127.0.0.1:8080`.

use super::*;

use axum::body::Body;
use axum::http::header::{COOKIE, HOST, LOCATION, ORIGIN};
use axum::http::{Method, Request};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::config::config::Role;

/// The address of the server in the tests.
const ORIGIN_VALUE: &str = "http://127.0.0.1:8080";

fn config(role: Role) -> Config {
    Config {
        role,
        single_host: false,
    }
}

/// The state of a server on port 8080 with the given role.
fn state(role: Role) -> AppState {
    AppState::new(config(role), 8080)
}

/// A request as the browser sends it, with an optional session.
fn request(method: Method, path: &str, session: Option<&str>, body: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method.clone())
        .uri(path)
        .header(HOST, "127.0.0.1:8080");

    if method != Method::GET {
        builder = builder
            .header(ORIGIN, ORIGIN_VALUE)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded");
    }

    if let Some(token) = session {
        builder = builder.header(COOKIE, format!("{}={token}", session::COOKIE_NAME));
    }

    builder.body(Body::from(body.to_owned())).unwrap()
}

/// Sends a request and returns the answer.
async fn send(state: &AppState, request: Request<Body>) -> Response {
    router(state.clone()).oneshot(request).await.unwrap()
}

/// The body of an answer as text.
async fn text(response: Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Logs in through the form and returns the session token.
async fn log_in(state: &AppState, name: &str) -> String {
    let response = send(state, request(Method::POST, "/login", None, &format!("username={name}&password=x"))).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let cookie = response.headers()[SET_COOKIE].to_str().unwrap();
    cookie
        .split(';')
        .next()
        .and_then(|pair| pair.split_once('='))
        .map(|(_, token)| token.to_owned())
        .unwrap()
}

#[tokio::test]
async fn pages_need_a_login() {
    let state = state(Role::Workstation);

    for path in ["/", "/chat", "/scan", "/settings"] {
        let response = send(&state, request(Method::GET, path, None, "")).await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(response.headers()[LOCATION], "/login", "{path}");
    }
}

#[tokio::test]
async fn the_login_page_is_open_to_everyone() {
    let state = state(Role::Workstation);

    let response = send(&state, request(Method::GET, "/login", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(text(response).await.contains("Mock-up login"));
}

#[tokio::test]
async fn a_logged_in_evaluator_sees_the_pages() {
    let state = state(Role::Workstation);
    let token = log_in(&state, "anna").await;

    let response = send(&state, request(Method::GET, "/scan", Some(&token), "")).await;

    assert_eq!(response.status(), StatusCode::OK);

    let html = text(response).await;

    assert!(html.contains("Scan and permissions"));
    assert!(html.contains("anna"));
    assert!(html.contains("No password reset is configured"));
}

#[tokio::test]
async fn the_home_page_depends_on_the_role() {
    for (role, home) in [(Role::Workstation, "/chat"), (Role::Master, "/admin")] {
        let state = state(role);
        let token = log_in(&state, "anna").await;

        let response = send(&state, request(Method::GET, "/", Some(&token), "")).await;

        assert_eq!(response.headers()[LOCATION], home, "{role:?}");
    }
}

#[tokio::test]
async fn each_role_refuses_the_pages_of_the_other() {
    let workstation = state(Role::Workstation);
    let token = log_in(&workstation, "anna").await;
    let response = send(&workstation, request(Method::GET, "/admin", Some(&token), "")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let master = state(Role::Master);
    let token = log_in(&master, "admin").await;

    for path in ["/chat", "/scan", "/settings"] {
        let response = send(&master, request(Method::GET, path, Some(&token), "")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn an_empty_name_is_refused() {
    let state = state(Role::Workstation);

    let response = send(&state, request(Method::POST, "/login", None, "username=+&password=")).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(text(response).await.contains("The name must have"));
}

#[tokio::test]
async fn logout_ends_the_session() {
    let state = state(Role::Workstation);
    let token = log_in(&state, "anna").await;

    let response = send(&state, request(Method::POST, "/logout", Some(&token), "")).await;

    assert_eq!(response.headers()[LOCATION], "/login");
    assert!(response.headers()[SET_COOKIE].to_str().unwrap().contains("Max-Age=0"));

    // The old token no longer opens a page.
    let response = send(&state, request(Method::GET, "/chat", Some(&token), "")).await;
    assert_eq!(response.headers()[LOCATION], "/login");
}

#[tokio::test]
async fn the_style_sheet_is_served() {
    let state = state(Role::Workstation);

    let response = send(&state, request(Method::GET, "/static/style.css", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()[CONTENT_TYPE].to_str().unwrap().starts_with("text/css"));
}

#[tokio::test]
async fn unknown_addresses_are_not_found() {
    let state = state(Role::Workstation);

    let response = send(&state, request(Method::GET, "/no-such-page", None, "")).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
