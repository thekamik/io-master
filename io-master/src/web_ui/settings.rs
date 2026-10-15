//! The settings page of a workstation: the users, the password,
//! and the API key for the AI.
//!
//! ```text
//! GET  /settings                  the page
//! POST /settings/users            add a user
//! POST /settings/password         change the own password
//! POST /settings/ai/key           check and store the API key
//! POST /settings/ai/key/delete    delete the API key
//! ```
//!
//! Every user of a workstation may add further users: all users
//! have equal rights (`docs/architecture.md`, section 3). The work
//! itself is done by `accounts/accounts.rs` and `ai/`; this file
//! turns forms into calls and results into pages.
//!
//! **The API key is never shown back**: not in the page, not in
//! a message, not even its last characters. The page tells only
//! whether a key is stored. A new key is checked with Anthropic
//! first and stored only when the check passes, so a stored key is
//! always one that worked.

use std::sync::Arc;

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Extension;
use serde::Deserialize;

use crate::accounts::accounts::{self, Account, AccountError};
use crate::ai::ai_error::AiError;
use crate::ai::ai_settings;
use crate::ai::api_key::ApiKey;
use crate::ai::model::Model;
use crate::db::database_key::DatabaseKey;
use crate::db::db::DbError;
use crate::db::user_db;
use crate::installation::lifecycle::Running;

use super::pages::{self, Page, escape};
use super::session::{CurrentUser, UserKey};
use super::web_ui::{AppState, not_found};

/// A message shown above the forms after a form was sent.
enum Notice {
    Done(String),
    Problem(String),
}

/// What the AI section shows: whether a key is stored, and for
/// which model; or why the database could not be read.
enum AiStatus {
    Known { has_key: bool, model: Model },
    Unknown(String),
}

/// The message after a key passed the check. It says what the key
/// is good for and nothing about the key itself.
fn configured(model: Model) -> String {
    format!(
        "The model {} is configured correctly and can be used in the chat or to support the evaluation.",
        model.title()
    )
}

/// Reads the AI status of the evaluator from their database.
async fn ai_status(state: &AppState, account: &Account, key: Arc<DatabaseKey>) -> AiStatus {
    let data_dir = state.data_dir.as_ref().clone();
    let account_id = account.id;

    let result = tokio::task::spawn_blocking(move || {
        let db = user_db::open(&data_dir, account_id, &key)?;

        Ok::<_, DbError>((ai_settings::has_api_key(&db)?, ai_settings::model(&db)?))
    })
    .await;

    match result {
        Ok(Ok((has_key, model))) => AiStatus::Known { has_key, model },
        Ok(Err(error)) => AiStatus::Unknown(format!("Your AI settings cannot be read: {error}.")),
        Err(_) => AiStatus::Unknown("Reading your AI settings was interrupted.".to_owned()),
    }
}

/// The HTML of the AI section: the state, the form for a key, and
/// the button that deletes it.
fn ai_section(status: &AiStatus) -> String {
    let (state, delete) = match status {
        AiStatus::Known { has_key: true, model } => (
            format!(
                "<p>An API key of Anthropic is stored. The chat uses {}; another model is chosen \
                 in the settings of the chat.</p>",
                escape(model.title())
            ),
            "<form method=\"post\" action=\"/settings/ai/key/delete\">\n\
             <button type=\"submit\">Delete the API key</button>\n\
             </form>"
                .to_owned(),
        ),
        AiStatus::Known { has_key: false, .. } => (
            "<p>No API key is stored: the chat cannot be used yet.</p>".to_owned(),
            String::new(),
        ),
        AiStatus::Unknown(problem) => (pages::problem_text(Some(problem)), String::new()),
    };

    format!(
        "<section>\n\
         <h2>AI</h2>\n\
         <p class=\"note\">The API key of Anthropic is kept only in your encrypted database, which \
         only your password opens. It is checked with Anthropic before it is stored, and it is \
         never shown again.</p>\n\
         {state}\n\
         <form method=\"post\" action=\"/settings/ai/key\" class=\"panel\">\n\
         <h3>Enter or replace the API key</h3>\n\
         <label>API key <input name=\"api_key\" type=\"password\" autocomplete=\"off\" \
         spellcheck=\"false\" required></label>\n\
         <button type=\"submit\">Check and store</button>\n\
         </form>\n\
         {delete}\n\
         </section>"
    )
}

/// The HTML of the page: the users, the form to add one, the form
/// to change the own password, and the AI section.
fn content(running: &Running, ai: &AiStatus, notice: Option<&Notice>) -> String {
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
         </section>\n\
         {ai}",
        ai = ai_section(ai),
    )
}

/// The whole page, with an optional notice and status.
async fn answer(
    state: &AppState,
    running: &Running,
    account: &Account,
    key: Arc<DatabaseKey>,
    status: StatusCode,
    notice: Option<Notice>,
) -> Response {
    let ai = ai_status(state, account, key).await;

    let html = pages::page(
        &running.installation,
        &account.name,
        Page::Settings,
        &content(running, &ai, notice.as_ref()),
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
pub async fn show(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
) -> Response {
    match workstation(&state) {
        Some(running) => answer(&state, &running, &account, key, StatusCode::OK, None).await,
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
    Extension(UserKey(key)): Extension<UserKey>,
    Form(form): Form<AddUserForm>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    if form.password != form.password_repeat {
        let notice = Notice::Problem("The two passwords differ.".to_owned());

        return answer(&state, &running, &account, key, StatusCode::BAD_REQUEST, Some(notice)).await;
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

    answer(&state, &running, &account, key, status, Some(notice)).await
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
    Extension(UserKey(key)): Extension<UserKey>,
    Form(form): Form<PasswordForm>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    if form.password != form.password_repeat {
        let notice = Notice::Problem("The two new passwords differ.".to_owned());

        return answer(&state, &running, &account, key, StatusCode::BAD_REQUEST, Some(notice)).await;
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

    answer(&state, &running, &account, key, status, Some(notice)).await
}

/// The field of the form with the API key.
#[derive(Deserialize)]
pub struct ApiKeyForm {
    api_key: String,
}

/// `POST /settings/ai/key`: checks the key with Anthropic, with the
/// model of the chat, and stores it when the check passes.
pub async fn store_api_key(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
    Form(form): Form<ApiKeyForm>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    // The text of the form is wiped by ApiKey::new.
    let api_key = match ApiKey::new(form.api_key) {
        Ok(api_key) => api_key,
        Err(error) => {
            let notice = Notice::Problem(format!("The key was not stored: {error}."));

            return answer(&state, &running, &account, key, StatusCode::BAD_REQUEST, Some(notice)).await;
        }
    };

    let (status, notice) = match check_and_store(&state, &account, Arc::clone(&key), api_key).await {
        Ok(model) => (StatusCode::OK, Notice::Done(configured(model))),
        Err(problem) => problem,
    };

    answer(&state, &running, &account, key, status, Some(notice)).await
}

/// The work of [`store_api_key`]: reads the model, checks the key
/// with it, stores the key. Returns the model, or the status and
/// notice of the failure.
async fn check_and_store(
    state: &AppState,
    account: &Account,
    key: Arc<DatabaseKey>,
    api_key: ApiKey,
) -> Result<Model, (StatusCode, Notice)> {
    let interrupted = || {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Notice::Problem("The work was interrupted; the key was not stored.".to_owned()),
        )
    };
    let not_saved = |error: DbError| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Notice::Problem(format!("The key was not stored: {error}.")),
        )
    };

    let data_dir = state.data_dir.as_ref().clone();
    let account_id = account.id;
    let reader_key = Arc::clone(&key);

    let model = tokio::task::spawn_blocking(move || {
        user_db::open(&data_dir, account_id, &reader_key).and_then(|db| ai_settings::model(&db))
    })
    .await
    .map_err(|_| interrupted())?
    .map_err(not_saved)?;

    state.ai.check(&api_key, model).await.map_err(|error| {
        let status = match error {
            AiError::NoConnection(_) | AiError::Overloaded | AiError::ServiceFailed(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::BAD_REQUEST,
        };

        (status, Notice::Problem(format!("The key was not stored: {error}.")))
    })?;

    let data_dir = state.data_dir.as_ref().clone();

    tokio::task::spawn_blocking(move || {
        user_db::open(&data_dir, account_id, &key).and_then(|db| ai_settings::store_api_key(&db, &api_key))
    })
    .await
    .map_err(|_| interrupted())?
    .map_err(not_saved)?;

    Ok(model)
}

/// `POST /settings/ai/key/delete`: deletes the API key.
pub async fn delete_api_key(
    State(state): State<AppState>,
    Extension(CurrentUser(account)): Extension<CurrentUser>,
    Extension(UserKey(key)): Extension<UserKey>,
) -> Response {
    let Some(running) = workstation(&state) else {
        return not_found().await;
    };

    let data_dir = state.data_dir.as_ref().clone();
    let account_id = account.id;
    let worker_key = Arc::clone(&key);

    let result = tokio::task::spawn_blocking(move || {
        user_db::open(&data_dir, account_id, &worker_key).and_then(|db| ai_settings::remove_api_key(&db))
    })
    .await;

    let (status, notice) = match result {
        Ok(Ok(())) => (StatusCode::OK, Notice::Done("The API key was deleted.".to_owned())),
        Ok(Err(error)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Notice::Problem(format!("The key was not deleted: {error}.")),
        ),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Notice::Problem("The work was interrupted.".to_owned())),
    };

    answer(&state, &running, &account, key, status, Some(notice)).await
}
