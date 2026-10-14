//! The web user interface: the server and its routes.
//!
//! The server listens on 127.0.0.1 over plain HTTP: the connection
//! between the browser and the server never leaves the computer.
//! The TLS server for `--address` comes in step 4 of
//! `docs/todo.md`.
//!
//! The work is split over the files of this folder:
//!
//! ```text
//! web_ui.rs     the server, the routes, login and logout
//! setup.rs      the first start: setup code, first user, role
//! settings.rs   the settings page: users and password
//! session.rs    sessions and the login check
//! security.rs   Host and Origin checks, protective headers
//! pages.rs      the HTML of the pages
//! static/       the style sheet and the logo
//! ```
//!
//! Until the installation is done, every address leads to the
//! setup page; afterwards the setup page is gone. Which pages
//! exist depends on the role chosen there: the pages of the other
//! role answer "not found".
//!
//! Work that is slow on purpose (Argon2id at login) runs on the
//! threads that tokio keeps for blocking work, so that the server
//! keeps answering meanwhile.

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

use crate::accounts::accounts::{self, AccountError};
use crate::installation::lifecycle::{Opened, Running};

use super::pages::{self, Page};
use super::security::{check_host, check_origin, security_headers};
use super::session::{self, CurrentUser, Sessions};
use super::settings;
use super::setup::{self, SetupCode};

/// The style sheet, built into the program.
const STYLE: &str = include_str!("static/style.css");

/// The logo, built into the program: a 192-pixel copy of
/// `static/logo.png` (the original, 1254 pixels), small enough to
/// load with every page.
const LOGO: &[u8] = include_bytes!("static/logo-small.png");

/// The address of the logo.
pub const LOGO_PATH: &str = "/static/logo.png";

/// What every request handler can reach.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Where the data of the installation is kept.
    pub data_dir: Arc<PathBuf>,
    /// The running installation; `None` until the first start is
    /// done. Written once, by the setup page.
    running: Arc<RwLock<Option<Arc<Running>>>>,
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
    pub fn new(data_dir: PathBuf, opened: Opened, setup_code: Option<SetupCode>, port: u16) -> AppState {
        let running = match opened {
            Opened::FirstRun => None,
            Opened::Installed(running) => Some(Arc::new(running)),
        };

        AppState {
            data_dir: Arc::new(data_dir),
            running: Arc::new(RwLock::new(running)),
            setup_code: setup_code.map(Arc::new),
            port,
            sessions: Sessions::default(),
        }
    }

    /// The running installation, or `None` before the first start
    /// is done.
    pub fn running(&self) -> Option<Arc<Running>> {
        // A panic while the lock was held cannot leave the value
        // half-written (it is replaced in one step), so a poisoned
        // lock is used as it is.
        self.running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Records the installation made by the setup page.
    pub fn install(&self, running: Running) {
        *self.running.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::new(running));
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
    // The pages, each one only for a logged-in user. Settings has
    // a page of its own; the others are placeholders for now.
    let mut pages = Router::new()
        .route("/", get(home))
        .route(Page::Settings.path(), get(settings::show))
        .route("/settings/users", post(settings::add_user))
        .route("/settings/password", post(settings::change_password));

    for page in [Page::Chat, Page::Scan, Page::Admin] {
        pages = pages.route(page.path(), get(move |state, user| show_placeholder(state, user, page)));
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
        .route(LOGO_PATH, get(logo))
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
    let installed = state.running().is_some();

    match (installed, path) {
        (true, "/setup") => Redirect::to("/").into_response(),
        (false, "/setup" | "/static/style.css") | (true, _) => next.run(request).await,
        (false, path) if path == LOGO_PATH => next.run(request).await,
        (false, _) => Redirect::to("/setup").into_response(),
    }
}

/// `/`: the first page of the role.
async fn home(State(state): State<AppState>) -> Response {
    match state.running() {
        Some(running) => Redirect::to(Page::home(running.installation.role).path()).into_response(),
        None => Redirect::to("/setup").into_response(),
    }
}

/// A page that is not built yet; "not found" for the pages of the
/// other role.
async fn show_placeholder(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    page: Page,
) -> Response {
    match state.running() {
        Some(running) if Page::of_role(running.installation.role).contains(&page) => {
            Html(pages::page(&running.installation, &account.name, page, page.placeholder())).into_response()
        }
        _ => not_found().await,
    }
}

/// The login page; a logged-in user goes straight on.
async fn login_form(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let logged_in = session::token_from(&headers)
        .and_then(|token| state.sessions.user(token))
        .is_some();

    match state.running() {
        Some(_) if logged_in => Redirect::to("/").into_response(),
        Some(running) => Html(pages::login_page(&running.installation, None)).into_response(),
        None => Redirect::to("/setup").into_response(),
    }
}

/// The fields of the login form.
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

/// Logs a user in: the password opens the key of the database of
/// the account (`accounts/accounts.rs`).
async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    let Some(running) = state.running() else {
        return Redirect::to("/setup").into_response();
    };

    let data_dir = state.data_dir.as_ref().clone();
    let worker = Arc::clone(&running);
    let name = form.username.trim().to_owned();

    // Argon2id takes a noticeable moment on purpose.
    let result = tokio::task::spawn_blocking(move || {
        accounts::log_in(&worker.system_db, worker.device_secret.as_ref(), &data_dir, &name, &form.password)
    })
    .await;

    let refuse = |status: StatusCode, problem: &str| {
        (status, Html(pages::login_page(&running.installation, Some(problem)))).into_response()
    };

    match result {
        Ok(Ok((account, key))) => match state.sessions.create(account, key) {
            Ok(token) => ([(SET_COOKIE, session::session_cookie(&token))], Redirect::to("/")).into_response(),
            Err(_) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "No session could be created."),
        },
        // One answer for a wrong name and a wrong password, so that
        // the page does not tell which names exist.
        Ok(Err(AccountError::WrongCredentials)) => refuse(StatusCode::UNAUTHORIZED, "Wrong name or password."),
        Ok(Err(error)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, &format!("The login failed: {error}.")),
        Err(_) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "The login was interrupted."),
    }
}

/// Logs the user out and ends the session; the key of the database
/// is wiped from memory.
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

/// The logo.
async fn logo() -> Response {
    ([(CONTENT_TYPE, "image/png")], LOGO).into_response()
}

/// Any other address, also the pages of the other role.
pub async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(pages::not_found_page())).into_response()
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "web_ui_tests.rs"]
mod tests;
