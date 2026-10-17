//! The first start, the login, the logout and the password.
//!
//! The work itself is done by `business/installation/lifecycle.rs` and
//! `business/session/accounts.rs`; these functions check what the page sent
//! and keep the session in [`UiState`].
//!
//! There is no one-time setup code, unlike in the web interface:
//! the code protected a network port against other programs and
//! accounts of the computer, and the desktop interface has no port.
//! Only the person in front of the window can make the setup.

use zeroize::Zeroizing;

use crate::business::installation::lifecycle::{self, LifecycleError};
use crate::business::session::accounts;
use crate::model::installation::Role;

use super::blocking::blocking;
use super::error::UiError;
use super::serializer::app_view::AppView;
use super::state::UiState;

/// The state of the program as the page shows it.
pub fn app_view(state: &UiState) -> AppView {
    let role = state.running().map(|running| running.installation.role);

    AppView::new(role, state.current_user().as_ref(), &state.data_dir)
}

/// The first start: makes the installation with its role and first
/// user, and logs that user in.
pub async fn set_up(
    state: &UiState,
    name: String,
    password: String,
    password_repeat: String,
    role: String,
    role_confirmed: bool,
) -> Result<AppView, UiError> {
    // The passwords are wiped from memory when they are dropped.
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    if state.running().is_some() {
        return Err(UiError::AlreadyInstalled);
    }

    if *password != *password_repeat {
        return Err(UiError::Invalid("the two passwords differ".to_owned()));
    }

    let role = Role::from_name(&role).ok_or_else(|| UiError::Invalid("choose a role".to_owned()))?;

    if !role_confirmed {
        return Err(UiError::Invalid("confirm that the role cannot be changed later".to_owned()));
    }

    // Argon2id is slow on purpose. The first user is logged in with
    // the same password, which opens the key of their database.
    let data_dir = state.data_dir.clone();
    let name = name.trim().to_owned();

    let (running, (account, key)) = blocking(move || {
        let (running, _) = lifecycle::install(&data_dir, role, &name, &password)?;
        let login =
            accounts::log_in(&running.system_db, None, &data_dir, &name, &password).map_err(LifecycleError::Account)?;

        Ok((running, login))
    })
    .await?;

    state.install(running);
    state.log_in(account, key);

    Ok(app_view(state))
}

/// Logs in with the name and the password.
pub async fn log_in(state: &UiState, name: String, password: String) -> Result<AppView, UiError> {
    let password = Zeroizing::new(password);
    let running = state.installed()?;
    let data_dir = state.data_dir.clone();
    let name = name.trim().to_owned();

    let (account, key) = blocking(move || {
        Ok(accounts::log_in(&running.system_db, running.device_secret.as_ref(), &data_dir, &name, &password)?)
    })
    .await?;

    state.log_in(account, key);

    Ok(app_view(state))
}

/// Logs out: ends the session and wipes the key of the database.
pub fn log_out(state: &UiState) -> AppView {
    state.log_out();

    app_view(state)
}

/// Changes the password of the logged-in evaluator.
pub async fn change_password(
    state: &UiState,
    current: String,
    password: String,
    password_repeat: String,
) -> Result<(), UiError> {
    let current = Zeroizing::new(current);
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    if *password != *password_repeat {
        return Err(UiError::Invalid("the two new passwords differ".to_owned()));
    }

    let running = state.installed()?;
    let session = state.session()?;

    blocking(move || {
        accounts::change_password(
            &running.system_db,
            running.device_secret.as_ref(),
            &session.account.name,
            &current,
            &password,
        )
        .map_err(|error| match error {
            // Here only the current password can be wrong.
            accounts::AccountError::WrongCredentials => UiError::Invalid("the current password is wrong".to_owned()),
            other => other.into(),
        })
    })
    .await
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "account_actions_tests.rs"]
mod tests;
