//! The command of the login form.

use tauri::State;
use zeroize::Zeroizing;

use crate::app::error::UiError;
use crate::app::state::UiState;
use crate::installation::{accounts, session::Session};

/// Logs in with the name and the password. A wrong password and an
/// unknown name give the same error (`UiError::WrongCredentials`), so
/// that the window does not tell which names exist.
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn login_form_log_in(state: State<'_, UiState>, name: String, password: String) -> Result<(), UiError> {
    let password = Zeroizing::new(password);
    let running = state.installed()?;

    let (account, key) =
        accounts::log_in(&running.system_db, running.device_secret.as_ref(), &state.data_dir, name.trim(), &password)?;

    state.log_in(Session::open(&state.data_dir, account, key)?);

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
