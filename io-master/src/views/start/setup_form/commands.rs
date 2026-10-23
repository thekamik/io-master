//! The command of the setup form.

use tauri::State;
use zeroize::Zeroizing;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::installation::lifecycle;
use crate::installation::{accounts, session::Session};
use crate::model::installation::Role;

/// The first start: makes the installation with `role` (`"workstation"`
/// or `"master"`) and its user `name`, and logs that user in. Refused
/// when the program is installed already, when the passwords differ,
/// when no role is chosen or its finality is not confirmed.
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works (twice: the key is made,
/// then opened by the login).
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn setup_form_install(
    state: State<'_, AppState>,
    name: String,
    password: String,
    password_repeat: String,
    role: String,
    role_confirmed: bool,
) -> Result<(), AppError> {
    // The passwords are wiped from memory when they are dropped.
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    ensure!(state.running().is_none(), AppError::AlreadyInstalled);
    ensure!(*password == *password_repeat, AppError::Invalid("the two passwords differ".to_owned()));

    let Some(role) = Role::from_name(&role) else {
        return Err(AppError::Invalid("choose a role".to_owned()));
    };

    ensure!(role_confirmed, AppError::Invalid("confirm that the role cannot be changed later".to_owned()));

    let name = name.trim();
    let (running, _) = lifecycle::install(&state.data_dir, role, name, &password)?;
    let (account, key) = accounts::log_in(&running.system_db, None, &state.data_dir, name, &password)?;
    let session = Session::open(&state.data_dir, account, key)?;

    state.install(running);
    state.log_in(session);

    Ok(())
}

// Unit tests, in their own file to keep this one readable.
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
