//! The first start: the first user and the role of the
//! installation.
//!
//! While nothing is installed, every page leads here. The page
//! asks for the **setup code**, a one-time code that the program
//! prints in the terminal where it was started. Only the person
//! who started the program sees it, so nobody else (another user
//! of the computer, another program) can make the choice first.
//!
//! The choice is made once: the installation is written only if
//! none exists (`installation.rs`), and afterwards this page is
//! no longer reachable.

use std::io;

use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

use crate::installation::installation::{self, InstallationError, Role};
use crate::random::random::fill_random;

use super::pages;
use super::session;
use super::web_ui::AppState;

/// The characters of a setup code: capital letters and digits
/// without those that are easily confused (0 and O, 1 and I).
/// There are 32 of them, so one random byte gives one character
/// without bias (`byte % 32`).
const CODE_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Characters in a setup code: 32^10 = 2^50 possible codes.
const CODE_LENGTH: usize = 10;

/// The one-time code that unlocks the first start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupCode(String);

impl SetupCode {
    /// Creates a new random code.
    pub fn generate() -> io::Result<SetupCode> {
        let mut bytes = [0u8; CODE_LENGTH];

        fill_random(&mut bytes)?;

        Ok(SetupCode(
            bytes
                .iter()
                .map(|byte| char::from(CODE_ALPHABET[usize::from(byte % 32)]))
                .collect(),
        ))
    }

    /// A code with known characters, for the tests of other
    /// modules (the tests play the person at the terminal, who
    /// knows the code).
    #[cfg(test)]
    pub fn from_text(text: &str) -> SetupCode {
        SetupCode(text.to_owned())
    }

    /// The code as it is shown: in two groups, `ABCDE-FGHJK`.
    pub fn display(&self) -> String {
        format!("{}-{}", &self.0[..CODE_LENGTH / 2], &self.0[CODE_LENGTH / 2..])
    }

    /// Tells whether a typed code is this one. Letter case,
    /// spaces and dashes do not matter.
    pub fn matches(&self, typed: &str) -> bool {
        let typed: String = typed
            .chars()
            .filter(|character| !matches!(character, ' ' | '-'))
            .map(|character| character.to_ascii_uppercase())
            .collect();

        // Every character is compared, also after a difference,
        // so the time of the comparison does not tell how much
        // of a guess was right.
        typed.len() == self.0.len()
            && typed
                .bytes()
                .zip(self.0.bytes())
                .fold(0u8, |difference, (left, right)| difference | (left ^ right))
                == 0
    }
}

/// The fields of the setup form.
#[derive(Deserialize)]
pub struct SetupForm {
    code: String,
    username: String,
    password: String,
    password_repeat: String,
    role: String,
    /// "I understand that the role cannot be changed."
    confirm: Option<String>,
}

/// The setup page.
pub async fn setup_form() -> Html<String> {
    Html(pages::setup_page(None))
}

/// Carries out the first start.
pub async fn setup(State(state): State<AppState>, Form(form): Form<SetupForm>) -> Response {
    let refuse = |status: StatusCode, problem: &str| {
        (status, Html(pages::setup_page(Some(problem)))).into_response()
    };

    let Some(code) = &state.setup_code else {
        // Not a first start; the router does not send anyone here.
        return Redirect::to("/").into_response();
    };

    if !code.matches(&form.code) {
        return refuse(
            StatusCode::FORBIDDEN,
            "The setup code is wrong. It is printed in the terminal where IO Master was started.",
        );
    }

    let user = form.username.trim();

    // TODO (step 5 of docs/todo.md): the password is checked here
    // but not kept; from step 5 it protects the database.
    if form.password.is_empty() || form.password != form.password_repeat {
        return refuse(StatusCode::BAD_REQUEST, "The two passwords are empty or differ.");
    }

    let Some(role) = Role::from_name(&form.role) else {
        return refuse(StatusCode::BAD_REQUEST, "Choose a role.");
    };

    if form.confirm.is_none() {
        return refuse(StatusCode::BAD_REQUEST, "Confirm that the role cannot be changed later.");
    }

    let created = installation::create(&state.data_dir, role, user);

    let installation = match created {
        Ok(installation) => installation,
        Err(InstallationError::InvalidUserName(reason)) => {
            return refuse(StatusCode::BAD_REQUEST, &format!("The name is not valid: {reason}."));
        }
        // Another request finished the setup a moment earlier.
        Err(InstallationError::AlreadyInstalled) => return Redirect::to("/login").into_response(),
        Err(error) => {
            return refuse(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("The installation could not be written: {error}."),
            );
        }
    };

    state.install(installation);

    // The first user is logged in straight away.
    match state.sessions.create(user) {
        Ok(token) => ([(SET_COOKIE, session::session_cookie(&token))], Redirect::to("/")).into_response(),
        Err(_) => Redirect::to("/login").into_response(),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
