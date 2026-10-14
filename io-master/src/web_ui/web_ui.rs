//! The web user interface: the server and its routes.
//!
//! The server listens only on 127.0.0.1, over plain HTTP: the
//! connection between the browser and the server never leaves the
//! computer, so there is nothing for TLS to protect. TLS is used
//! where data does leave the computer: between a workstation and
//! the master (step 7 of `docs/todo.md`).
//!
//! The work is split over the files of this folder:
//!
//! ```text
//! web_ui.rs     the server, the routes, the handlers
//! session.rs    login (mock-up), sessions, the login check
//! security.rs   Host and Origin checks, protective headers
//! pages.rs      the HTML of the pages
//! static/       the style sheet
//! ```
//!
//! Which pages exist depends on the role of the installation: the
//! routes of the other role are not registered at all, so they
//! answer "not found".

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::State;
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Extension, Form, Router};
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::config::config::Config;

use super::pages::{self, Page};
use super::security::{check_host, check_origin, security_headers};
use super::session::{self, CurrentUser, Sessions};

/// The style sheet, built into the program.
const STYLE: &str = include_str!("static/style.css");

/// The longest name of an evaluator, in characters.
const USER_NAME_MAX: usize = 64;

/// What every request handler can reach.
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// The port the server listens on; the names a browser may
    /// use for the server depend on it (security.rs).
    pub port: u16,
    pub sessions: Sessions,
}

impl AppState {
    pub fn new(config: Config, port: u16) -> AppState {
        AppState {
            config: Arc::new(config),
            port,
            sessions: Sessions::default(),
        }
    }
}

/// Opens the listening socket on 127.0.0.1 and the given port.
/// Kept apart from [`run`] so that the caller can report a busy
/// port before the server starts.
pub async fn bind(port: u16) -> io::Result<TcpListener> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await
}

/// Serves the web interface until `shutdown` completes, for
/// example when Ctrl+C is pressed.
pub async fn run(
    listener: TcpListener,
    config: Config,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    let port = listener.local_addr()?.port();

    axum::serve(listener, router(AppState::new(config, port)))
        .with_graceful_shutdown(shutdown)
        .await
}

/// All routes of the interface, with their protection.
pub fn router(state: AppState) -> Router {
    let home = Page::home(state.config.role).path();

    // The pages of the role, each one only for a logged-in
    // evaluator.
    let mut pages = Router::new().route("/", get(move || async move { Redirect::to(home) }));

    for &page in Page::of_role(state.config.role) {
        pages = pages.route(page.path(), get(move |state, user| show_page(state, user, page)));
    }

    let pages = pages
        .route("/logout", post(logout))
        .route_layer(from_fn_with_state(state.clone(), session::require_login));

    // The layers run from the last added to the first: the
    // headers are added to every answer, also to refusals; the
    // Host is checked before anything else.
    Router::new()
        .merge(pages)
        .route("/login", get(login_form).post(login))
        .route("/static/style.css", get(style))
        .fallback(not_found)
        .layer(from_fn_with_state(state.clone(), check_origin))
        .layer(from_fn_with_state(state.clone(), check_host))
        .layer(from_fn(security_headers))
        .with_state(state)
}

/// A page of the navigation.
async fn show_page(
    State(state): State<AppState>,
    Extension(CurrentUser(user)): Extension<CurrentUser>,
    page: Page,
) -> Html<String> {
    Html(pages::page(&state.config, &user, page))
}

/// The login page; a logged-in evaluator goes straight on.
async fn login_form(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let logged_in = session::token_from(&headers)
        .and_then(|token| state.sessions.user(token))
        .is_some();

    if logged_in {
        return Redirect::to("/").into_response();
    }

    Html(pages::login_page(&state.config, None)).into_response()
}

/// The fields of the login form.
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    // Read but not checked: the login is a mock-up (session.rs).
    #[allow(dead_code)]
    password: String,
}

/// Logs an evaluator in. Mock-up: any name is accepted.
async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let user = form.username.trim();

    if user.is_empty() || user.chars().count() > USER_NAME_MAX {
        let problem = format!("The name must have 1 to {USER_NAME_MAX} characters.");

        return (
            StatusCode::BAD_REQUEST,
            Html(pages::login_page(&state.config, Some(&problem))),
        )
            .into_response();
    }

    match state.sessions.create(user) {
        Ok(token) => (
            [(SET_COOKIE, session::session_cookie(&token))],
            Redirect::to("/"),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(pages::login_page(&state.config, Some("No session could be created."))),
        )
            .into_response(),
    }
}

/// Logs the evaluator out and ends the session.
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = session::token_from(&headers) {
        state.sessions.remove(token);
    }

    ([(SET_COOKIE, session::expired_cookie())], Redirect::to("/login")).into_response()
}

/// The style sheet.
async fn style() -> Response {
    ([(CONTENT_TYPE, "text/css; charset=utf-8")], STYLE).into_response()
}

/// Any other address, also the pages of the other role.
async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(pages::not_found_page())).into_response()
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "web_ui_tests.rs"]
mod tests;
