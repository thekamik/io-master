//! The settings page of a workstation: the users and the password.
//!
//! ```text
//! GET  /settings            the page
//! POST /settings/users      add a user
//! POST /settings/password   change the own password
//! ```
//!
//! Every user of a workstation may add further users: all users
//! have equal rights (`docs/architecture.md`, section 3). The work
//! itself is done by `accounts/accounts.rs`; this file turns forms
//! into calls and results into pages.

use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Extension;
use serde::Deserialize;

use crate::accounts::accounts::{self, Account, AccountError};
use crate::installation::lifecycle::Running;

use super::pages::{self, Page, escape};
use super::session::CurrentUser;
use super::web_ui::{AppState, not_found};

/// A message shown above the forms after a form was sent.
enum Notice {
    Done(String),
    Problem(String),
}

/// The HTML of the page: the users, the form to add one, and the
/// form to change the own password.
fn content(running: &Running, notice: Option<&Notice>) -> String {
    let notice = match notice {
        Some(Notice::Done(text)) => format!("<p class=\"done\">{}</p>", escape(text)),
        Some(Notice::Problem(text)) => pages::problem_text(Some(text)),
        None => String::new(),
    };

    let users: String = match running.system_db.user_names() {
        Ok(names) => names.iter().map(|name| format!("<li>{}</li>", escape(name))).collect(),
        Err(error) => format!("<li>{}</li>", escape(&format!("The users cannot be read: {error}"))),
    };

    format!(
        "{notice}\n\
         <section>\n\
         <h2>Users</h2>\n\
         <p class=\"note\">All users of this workstation have equal rights. Each has \
         a database of their own that only their password opens.</p>\n\
         <ul>{users}</ul>\n\
         <form method=\"post\" action=\"/settings/users\" class=\"panel\">\n\
         <h3>Add a user</h3>\n\
         <label>Name <input name=\"username\" autocomplete=\"off\" required></label>\n\
         <label>Password <input name=\"password\" type=\"password\" autocomplete=\"new-password\" \
         minlength=\"12\" required></label>\n\
         <label>Password again <input name=\"password_repeat\" type=\"password\" \
         autocomplete=\"new-password\" minlength=\"12\" required></label>\n\
         <button type=\"submit\">Add user</button>\n\
         </form>\n\
         </section>\n\
         <section>\n\
         <h2>Your password</h2>\n\
         <form method=\"post\" action=\"/settings/password\" class=\"panel\">\n\
         <label>Current password <input name=\"current\" type=\"password\" \
         autocomplete=\"current-password\" required></label>\n\
         <label>New password <input name=\"password\" type=\"password\" autocomplete=\"new-password\" \
         minlength=\"12\" required></label>\n\
         <label>New password again <input name=\"password_repeat\" type=\"password\" \
         autocomplete=\"new-password\" minlength=\"12\" required></label>\n\
         <button type=\"submit\">Change password</button>\n\
         </form>\n\
         </section>"
    )
}

/// The whole page, with an optional notice and status.
fn answer(running: &Running, account: &Account, status: StatusCode, notice: Option<Notice>) -> Response {
    let html = pages::page(
        &running.installation,
        &account.name,
        Page::Settings,
        &content(running, notice.as_ref()),
    );

    (status, Html(html)).into_response()
}

/// The running installation, when it is a workstation: only a
/// workstation has this page.
fn workstation(state: &AppState) -> Option<Arc<Running>> {
    state
        .running()
        .filter(|running| Page::of_role(running.installation.role).contains(&Page::Settings))
}

/// `GET /settings`.
pub async fn show(State(state): State<AppState>, Extension(CurrentUser(account)): Extension<CurrentUser>) -> Response {
    match workstation(&state) {
        Some(running) => answer(&running, &account, StatusCode::OK, None),
        None => not_found().await,
    }
}

/// The fields of the form that adds a user.
#[derive(Deserialize)]
pub struct AddUserForm {
    username: String,
    password: String,
    password_repeat: String,
}

/// `POST /settings/users`: adds a user.
pub async fn add_user(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Form(form): Form<AddUserForm>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    if form.password != form.password_repeat {
        let notice = Notice::Problem("The two passwords differ.".to_owned());

        return answer(&running, &account, StatusCode::BAD_REQUEST, Some(notice));
    }

    let name = form.username.trim().to_owned();
    let data_dir = state.data_dir.as_ref().clone();
    let worker = Arc::clone(&running);
    let new_name = name.clone();

    // Argon2id and the new database take a moment.
    let result = tokio::task::spawn_blocking(move || {
        accounts::add(&worker.system_db, worker.device_secret.as_ref(), &data_dir, &new_name, &form.password)
    })
    .await;

    let (status, notice) = match result {
        Ok(Ok(added)) => (StatusCode::OK, Notice::Done(format!("The user {} was added.", added.name))),
        Ok(Err(error @ (AccountError::InvalidName(_) | AccountError::NameTaken | AccountError::WeakPassword(_)))) => {
            (StatusCode::BAD_REQUEST, Notice::Problem(format!("The user was not added: {error}.")))
        }
        Ok(Err(error)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Notice::Problem(format!("The user {name} was not added: {error}.")),
        ),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Notice::Problem("The work was interrupted.".to_owned())),
    };

    answer(&running, &account, status, Some(notice))
}

/// The fields of the form that changes the password.
#[derive(Deserialize)]
pub struct PasswordForm {
    current: String,
    password: String,
    password_repeat: String,
}

/// `POST /settings/password`: changes the password of the logged-in
/// user.
pub async fn change_password(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Form(form): Form<PasswordForm>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    if form.password != form.password_repeat {
        let notice = Notice::Problem("The two new passwords differ.".to_owned());

        return answer(&running, &account, StatusCode::BAD_REQUEST, Some(notice));
    }

    let worker = Arc::clone(&running);
    let name = account.name.clone();

    let result = tokio::task::spawn_blocking(move || {
        accounts::change_password(&worker.system_db, worker.device_secret.as_ref(), &name, &form.current, &form.password)
    })
    .await;

    let (status, notice) = match result {
        Ok(Ok(())) => (StatusCode::OK, Notice::Done("Your password was changed.".to_owned())),
        Ok(Err(AccountError::WrongCredentials)) => (
            StatusCode::BAD_REQUEST,
            Notice::Problem("The current password is wrong.".to_owned()),
        ),
        Ok(Err(error @ AccountError::WeakPassword(_))) => (
            StatusCode::BAD_REQUEST,
            Notice::Problem(format!("The password was not changed: {error}.")),
        ),
        Ok(Err(error)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Notice::Problem(format!("The password was not changed: {error}.")),
        ),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Notice::Problem("The work was interrupted.".to_owned())),
    };

    answer(&running, &account, status, Some(notice))
}
