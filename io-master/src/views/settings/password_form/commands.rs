//! The command of the password form.

use tauri::State;
use zeroize::Zeroizing;

use crate::installation::accounts::{self, AccountError};
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

/// Changes the password of the logged-in evaluator. The current
/// password is needed; the two new ones must agree.
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn password_form_change(
    state: State<'_, UiState>,
    current: String,
    password: String,
    password_repeat: String,
) -> Result<(), UiError> {
    // The passwords are wiped from memory when they are dropped.
    let current = Zeroizing::new(current);
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    ensure!(*password == *password_repeat, UiError::Invalid("the two new passwords differ".to_owned()));

    let running = state.installed()?;
    let session = state.session()?;

    let changed = accounts::change_password(
        &running.system_db,
        running.device_secret.as_ref(),
        &session.account.name,
        &current,
        &password,
    );

    match changed {
        Ok(()) => Ok(()),
        // Here only the current password can be wrong.
        Err(AccountError::WrongCredentials) => Err(UiError::Invalid("the current password is wrong".to_owned())),
        Err(other) => Err(other.into()),
    }
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
