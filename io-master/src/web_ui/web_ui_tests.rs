//! Unit tests of `web_ui.rs` and `setup.rs`: the routes, as
//! a browser sees them.
//!
//! The requests go straight to the router, without a network
//! port. Every request carries the headers a browser would send
//! to `http://127.0.0.1:8080`.

use super::*;

use axum::body::Body;
use axum::http::header::{COOKIE, HOST, LOCATION, ORIGIN};
use axum::http::{Method, Request as HttpRequest};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::installation::installation::Role;
use crate::installation::lifecycle;
use crate::tpm::test_support::TestTpm;

/// The address of the server in the tests.
const ORIGIN_VALUE: &str = "http://127.0.0.1:8080";

/// The setup code of the first starts in the tests.
const CODE: &str = "ABCDEFGHJK";

/// A server whose installation is done, with the given users. The
/// routes tested with it never reach the TPM.
fn installed(role: Role, users: &[&str]) -> AppState {
    let installation = Installation {
        id: [1; 16],
        role,
        users: users.iter().map(|user| user.to_string()).collect(),
    };

    AppState::new(
        std::env::temp_dir(),
        InstallationState::Installed(installation),
        None,
        8080,
        TpmAccess { port: 0 },
    )
}

/// A server at its first start, in the data directory of a test
/// TPM, with the setup code [`CODE`].
fn first_start(test: &TestTpm) -> AppState {
    AppState::new(
        test.dir().to_owned(),
        InstallationState::FirstRun,
        Some(SetupCode::from_text(CODE)),
        8080,
        test.access(),
    )
}

/// The state of the installation as the next start would find it.
fn state_of(test: &TestTpm) -> InstallationState {
    lifecycle::open(test.dir(), &mut test.connect()).unwrap()
}

/// A request as the browser sends it, with an optional session.
fn request(method: Method, path: &str, session: Option<&str>, body: &str) -> HttpRequest<Body> {
    let mut builder = HttpRequest::builder()
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
async fn send(state: &AppState, request: HttpRequest<Body>) -> Response {
    router(state.clone()).oneshot(request).await.unwrap()
}

/// The body of an answer as text.
async fn text(response: Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The session token set by an answer.
fn token_of(response: &Response) -> String {
    response.headers()[SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .and_then(|pair| pair.split_once('='))
        .map(|(_, token)| token.to_owned())
        .unwrap()
}

/// Logs in through the form and returns the session token.
async fn log_in(state: &AppState, name: &str) -> String {
    let response = send(state, request(Method::POST, "/login", None, &format!("username={name}&password=x"))).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    token_of(&response)
}

/// The fields of a complete setup form, with the given role.
fn setup_form(role: &str) -> String {
    format!(
        "code={CODE}&username=anna&password=secret&password_repeat=secret&role={role}&confirm=on"
    )
}

/* ---------------------------------------------------------------
 * The first start
 * ------------------------------------------------------------- */

#[tokio::test]
async fn before_the_installation_everything_leads_to_the_setup() {
    let test = TestTpm::new();
    let state = first_start(&test);

    for path in ["/", "/chat", "/login", "/admin", "/no-such-page"] {
        let response = send(&state, request(Method::GET, path, None, "")).await;

        assert_eq!(response.headers()[LOCATION], "/setup", "{path}");
    }

    let response = send(&state, request(Method::GET, "/setup", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(text(response).await.contains("Setup code"));
}

#[tokio::test]
async fn a_wrong_setup_code_is_refused() {
    let test = TestTpm::new();
    let state = first_start(&test);

    let form = setup_form("workstation").replace(CODE, "WRONGCODE2");
    let response = send(&state, request(Method::POST, "/setup", None, &form)).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(state_of(&test), InstallationState::FirstRun);
}

#[tokio::test]
async fn an_incomplete_setup_is_refused() {
    let test = TestTpm::new();
    let state = first_start(&test);

    for form in [
        setup_form("workstation").replace("password_repeat=secret", "password_repeat=other"),
        setup_form("workstation").replace("&confirm=on", ""),
        setup_form("admin"),
        setup_form("workstation").replace("username=anna", "username=+"),
    ] {
        let response = send(&state, request(Method::POST, "/setup", None, &form)).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{form}");
    }

    assert_eq!(state_of(&test), InstallationState::FirstRun);
}

#[tokio::test]
async fn the_setup_installs_and_logs_the_first_user_in() {
    let test = TestTpm::new();
    let state = first_start(&test);

    let response = send(&state, request(Method::POST, "/setup", None, &setup_form("workstation"))).await;

    assert_eq!(response.headers()[LOCATION], "/");

    // The installation is written and the first user is in.
    let InstallationState::Installed(written) = state_of(&test) else {
        panic!("nothing was installed");
    };
    assert_eq!(written.role, Role::Workstation);
    assert_eq!(written.users, ["anna"]);

    let token = token_of(&response);
    let response = send(&state, request(Method::GET, "/chat", Some(&token), "")).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn after_the_setup_the_choice_is_gone() {
    let test = TestTpm::new();
    let state = first_start(&test);

    send(&state, request(Method::POST, "/setup", None, &setup_form("master"))).await;

    // The setup page leads home, and a second setup changes nothing.
    let response = send(&state, request(Method::GET, "/setup", None, "")).await;
    assert_eq!(response.headers()[LOCATION], "/");

    let response = send(&state, request(Method::POST, "/setup", None, &setup_form("workstation"))).await;
    assert_eq!(response.headers()[LOCATION], "/");

    assert!(matches!(
        state_of(&test),
        InstallationState::Installed(written) if written.role == Role::Master
    ));
}

/* ---------------------------------------------------------------
 * An installed program
 * ------------------------------------------------------------- */

#[tokio::test]
async fn pages_need_a_login() {
    let state = installed(Role::Workstation, &["anna"]);

    for path in ["/", "/chat", "/scan", "/settings"] {
        let response = send(&state, request(Method::GET, path, None, "")).await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(response.headers()[LOCATION], "/login", "{path}");
    }
}

#[tokio::test]
async fn only_users_of_the_installation_log_in() {
    let state = installed(Role::Workstation, &["anna"]);

    let response = send(&state, request(Method::POST, "/login", None, "username=mallory&password=x")).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(text(response).await.contains("Unknown user."));
}

#[tokio::test]
async fn a_logged_in_user_sees_the_pages() {
    let state = installed(Role::Workstation, &["anna"]);
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
        let state = installed(role, &["anna"]);
        let token = log_in(&state, "anna").await;

        let response = send(&state, request(Method::GET, "/", Some(&token), "")).await;

        assert_eq!(response.headers()[LOCATION], home, "{role:?}");
    }
}

#[tokio::test]
async fn each_role_refuses_the_pages_of_the_other() {
    let workstation = installed(Role::Workstation, &["anna"]);
    let token = log_in(&workstation, "anna").await;
    let response = send(&workstation, request(Method::GET, "/admin", Some(&token), "")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let master = installed(Role::Master, &["admin"]);
    let token = log_in(&master, "admin").await;

    for path in ["/chat", "/scan", "/settings"] {
        let response = send(&master, request(Method::GET, path, Some(&token), "")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn logout_ends_the_session() {
    let state = installed(Role::Workstation, &["anna"]);
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
    let state = installed(Role::Workstation, &["anna"]);

    let response = send(&state, request(Method::GET, "/static/style.css", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()[CONTENT_TYPE].to_str().unwrap().starts_with("text/css"));
}

#[tokio::test]
async fn the_logo_is_served_also_before_the_installation() {
    let test = TestTpm::new();

    for state in [installed(Role::Workstation, &["anna"]), first_start(&test)] {
        let response = send(&state, request(Method::GET, LOGO_PATH, None, "")).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/png");

        // A PNG file starts with these eight bytes.
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}

#[tokio::test]
async fn unknown_addresses_are_not_found() {
    let state = installed(Role::Workstation, &["anna"]);

    let response = send(&state, request(Method::GET, "/no-such-page", None, "")).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
