//! Help for the tests of the web interface: a server with a real
//! installation (databases, accounts) in a temporary directory, and
//! requests as a browser sends them.

use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, HOST, ORIGIN, SET_COOKIE};
use axum::http::{Method, Request};
use axum::response::Response;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::accounts::accounts;
use crate::installation::installation::Role;
use crate::installation::lifecycle::{self, Opened};
use crate::tpm::test_support::TempDir;

use super::session;
use super::setup::SetupCode;
use super::web_ui::{AppState, router};

/// The password of every user of the tests.
pub const PASSWORD: &str = "correct horse battery";

/// The setup code of the first starts in the tests.
pub const CODE: &str = "ABCDEFGHJK";

/// A server and its data directory; dropping it removes the
/// directory.
pub struct TestServer {
    pub directory: TempDir,
    pub state: AppState,
}

/// A server whose installation is done, with the given users (the
/// first one installed it), all with [`PASSWORD`].
pub fn installed(role: Role, users: &[&str]) -> TestServer {
    let directory = TempDir::new();

    let (running, _) =
        lifecycle::install(directory.path(), role, users[0], PASSWORD).expect("the installation is made");

    for user in &users[1..] {
        accounts::add(&running.system_db, None, directory.path(), user, PASSWORD).expect("the user is added");
    }

    let state = AppState::new(directory.path().to_owned(), Opened::Installed(running), None, 8080);

    TestServer { directory, state }
}

/// A server at its first start, with the setup code [`CODE`].
pub fn first_start() -> TestServer {
    let directory = TempDir::new();

    let state = AppState::new(
        directory.path().to_owned(),
        Opened::FirstRun,
        Some(SetupCode::from_text(CODE)),
        8080,
    );

    TestServer { directory, state }
}

/// A request as the browser sends it to `http://127.0.0.1:8080`,
/// with an optional session.
pub fn request(method: Method, path: &str, session: Option<&str>, body: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method.clone())
        .uri(path)
        .header(HOST, "127.0.0.1:8080");

    if method != Method::GET {
        builder = builder
            .header(ORIGIN, "http://127.0.0.1:8080")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded");
    }

    if let Some(token) = session {
        builder = builder.header(COOKIE, format!("{}={token}", session::COOKIE_NAME));
    }

    builder.body(Body::from(body.to_owned())).unwrap()
}

/// Sends a request and returns the answer.
pub async fn send(state: &AppState, request: Request<Body>) -> Response {
    router(state.clone()).oneshot(request).await.unwrap()
}

/// The body of an answer as text.
pub async fn text(response: Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The session token set by an answer.
pub fn token_of(response: &Response) -> String {
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
pub async fn log_in(state: &AppState, name: &str) -> String {
    let body = format!("username={name}&password={}", PASSWORD.replace(' ', "+"));
    let response = send(state, request(Method::POST, "/login", None, &body)).await;

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER, "login of {name}");

    token_of(&response)
}
