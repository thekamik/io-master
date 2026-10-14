//! The web user interface: the server and its routes.
//!
//! The server listens on 127.0.0.1 over plain HTTP: the connection
//! between the browser and the server never leaves the computer.
//! The TLS server for `--address` comes in step 5a of
//! `docs/todo.md`.
//!
//! The work is split over the files of this folder:
//!
//! ```text
//! web_ui.rs     the server, the routes, the handlers
//! setup.rs      the first start: setup code, first user, role
//! session.rs    login (mock-up), sessions, the login check
//! security.rs   Host and Origin checks, protective headers
//! pages.rs      the HTML of the pages
//! static/       the style sheet
//! ```
//!
//! Until the installation is done, every address leads to the
//! setup page; afterwards the setup page is gone. Which pages
//! exist depends on the role chosen there: the pages of the other
//! role answer "not found".

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::extract::{Request, State};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{Next, from_fn, from_fn_with_state};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Extension, Form, Router};
use serde::Deserialize;
use tokio::net::TcpListener;

use crate::installation::installation::{Installation, State as InstallationState};

use super::pages::{self, Page};
use super::security::{check_host, check_origin, security_headers};
use super::session::{self, CurrentUser, Sessions};
use super::setup::{self, SetupCode};

/// The style sheet, built into the program.
const STYLE: &str = include_str!("static/style.css");

/// What every request handler can reach.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Where the data of the installation is kept.
    pub data_dir: Arc<PathBuf>,
    /// The installation; `None` until the first start is done.
    /// Written once, by the setup page.
    installation: Arc<RwLock<Option<Installation>>>,
    /// The one-time code of the first start; `None` when the
    /// program is installed.
    pub setup_code: Option<Arc<SetupCode>>,
    /// The port the server listens on; the names a browser may
    /// use for the server depend on it (security.rs).
    pub port: u16,
    pub sessions: Sessions,
}

impl AppState {
    /// The state of a server. `setup_code` is needed only for the
    /// first start.
    pub fn new(data_dir: PathBuf, state: InstallationState, setup_code: Option<SetupCode>, port: u16) -> AppState {
        let installation = match state {
            InstallationState::FirstRun => None,
            InstallationState::Installed(installation) => Some(installation),
        };

        AppState {
            data_dir: Arc::new(data_dir),
            installation: Arc::new(RwLock::new(installation)),
            setup_code: setup_code.map(Arc::new),
            port,
            sessions: Sessions::default(),
        }
    }

    /// The installation, or `None` before the first start is done.
    pub fn installation(&self) -> Option<Installation> {
        // A panic while the lock was held cannot leave the value
        // half-written (it is replaced in one step), so a poisoned
        // lock is used as it is.
        self.installation
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Records the installation made by the setup page.
    pub fn install(&self, installation: Installation) {
        *self
            .installation
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(installation);
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
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await
}

/// All routes of the interface, with their protection.
pub fn router(state: AppState) -> Router {
    // The pages, each one only for a logged-in user.
    let mut pages = Router::new().route("/", get(home));

    for page in Page::ALL {
        pages = pages.route(page.path(), get(move |state, user| show_page(state, user, page)));
    }

    let pages = pages
        .route("/logout", post(logout))
        .route_layer(from_fn_with_state(state.clone(), session::require_login));

    // The layers run from the last added to the first: the
    // headers are added to every answer, also to refusals; the
    // Host is checked before anything else; nothing but the setup
    // page is reachable before the installation is done.
    Router::new()
        .merge(pages)
        .route("/setup", get(setup::setup_form).post(setup::setup))
        .route("/login", get(login_form).post(login))
        .route("/static/style.css", get(style))
        .fallback(not_found)
        .layer(from_fn_with_state(state.clone(), require_installed))
        .layer(from_fn_with_state(state.clone(), check_origin))
        .layer(from_fn_with_state(state.clone(), check_host))
        .layer(from_fn(security_headers))
        .with_state(state)
}

/// Before the first start is done every address leads to the setup
/// page; afterwards the setup page leads home.
async fn require_installed(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let installed = state.installation().is_some();

    match (installed, path) {
        (true, "/setup") => Redirect::to("/").into_response(),
        (false, "/setup" | "/static/style.css") | (true, _) => next.run(request).await,
        (false, _) => Redirect::to("/setup").into_response(),
    }
}

/// `/`: the first page of the role.
async fn home(State(state): State<AppState>) -> Response {
    match state.installation() {
        Some(installation) => Redirect::to(Page::home(installation.role).path()).into_response(),
        None => Redirect::to("/setup").into_response(),
    }
}

/// A page of the navigation; "not found" for the pages of the
/// other role.
async fn show_page(
    State(state): State<AppState>,
    Extension(CurrentUser(user)): Extension<CurrentUser>,
    page: Page,
) -> Response {
    match state.installation() {
        Some(installation) if Page::of_role(installation.role).contains(&page) => {
            Html(pages::page(&installation, &user, page)).into_response()
        }
        _ => not_found().await,
    }
}

/// The login page; a logged-in user goes straight on.
async fn login_form(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let logged_in = session::token_from(&headers)
        .and_then(|token| state.sessions.user(token))
        .is_some();

    match state.installation() {
        Some(_) if logged_in => Redirect::to("/").into_response(),
        Some(installation) => Html(pages::login_page(&installation, None)).into_response(),
        None => Redirect::to("/setup").into_response(),
    }
}

/// The fields of the login form.
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    // Read but not checked yet (see login()).
    #[allow(dead_code)]
    password: String,
}

/// Logs a user in.
///
/// TODO (step 5 of docs/todo.md): mock-up. The user must exist,
/// but the password is not checked yet.
async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let Some(installation) = state.installation() else {
        return Redirect::to("/setup").into_response();
    };

    let user = form.username.trim();

    if !installation.has_user(user) {
        return (
            StatusCode::UNAUTHORIZED,
            Html(pages::login_page(&installation, Some("Unknown user."))),
        )
            .into_response();
    }

    match state.sessions.create(user) {
        Ok(token) => ([(SET_COOKIE, session::session_cookie(&token))], Redirect::to("/")).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(pages::login_page(&installation, Some("No session could be created."))),
        )
            .into_response(),
    }
}

/// Logs the user out and ends the session.
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
