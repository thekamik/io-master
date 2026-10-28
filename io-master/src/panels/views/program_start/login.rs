//! The login, as the page calls it (`ui/src/views/program_start/`): what
//! the window shows at its start (the first start, the login, or the user
//! logged in), the login with a name and a password, and the logout.
//!
//! ```text
//! App.svelte  ──login_get()──────────────► program_start::installed_user, Session ──► ProgramView
//! LoginForm   ──login_log_in(name, pw)───► program_start::log_in ──► Session::log_in
//! the frame   ──login_log_out()──────────► Session::log_out
//! ```

use serde::Serialize;
use tauri::State;
use zeroize::Zeroizing;

use crate::panels::logic::program_start::access::{installed_user, log_in};
use crate::panels::views::error::ViewError;
use crate::session::state::Session;

/// What the window shows at its start.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramView {
    /// Whether the program is installed: if not, the first start.
    installed: bool,
    /// The name of the user logged in; `None`: the login (or the first start).
    logged_in: Option<String>,
}

/// The state of the program for the page: installed or not, and who is
/// logged in.
///
/// Called by: `App.svelte`, when the window opens and after every change.
/// Calls: `program_start::access::installed_user`, `Session::lock`.
#[tauri::command]
pub fn login_get(session: State<'_, Session>) -> Result<ProgramView, ViewError> {
    let installed = installed_user(session.data_folder())?.is_some();
    let logged_in = session.lock()?.as_ref().map(|opened| opened.user.name.clone());

    Ok(ProgramView { installed, logged_in })
}

/// Logs the user in with their name and password, and keeps their open
/// database in the session. Run apart from the window (`async`): unwrapping
/// the key takes a moment by design (Argon2id). The password is wiped from
/// memory when the command ends.
///
/// Called by: `LoginForm.svelte`, "Log in".
/// Calls: `program_start::access::log_in`, `Session::log_in`.
#[tauri::command(async)]
pub fn login_log_in(session: State<'_, Session>, name: String, password: String) -> Result<(), ViewError> {
    let password = Zeroizing::new(password);
    let (user, database) = log_in(session.data_folder(), &name, &password)?;

    session.log_in(user, database)?;

    Ok(())
}

/// Logs the user out: their database is closed.
///
/// Called by: the frame of the window (`App.svelte`), "Log out".
/// Calls: `Session::log_out`.
#[tauri::command]
pub fn login_log_out(session: State<'_, Session>) -> Result<(), ViewError> {
    session.log_out()?;

    Ok(())
}
