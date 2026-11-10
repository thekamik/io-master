//! The login, as the page calls it (`ui/src/views/program_start/`): what
//! the window shows at its start (the first start, the login, or the user
//! logged in), the login with a name and a password, the recovery of a
//! forgotten password with the recovery code, and the logout.
//!
//! ```text
//! App.svelte  ──login_get()──────────────► program_start::installed_user, Session ──► ProgramView
//! LoginForm   ──login_log_in(name, pw)───► user_data_rules::log_in ──► Session::log_in
//! LoginForm   ──login_recover(name, code, pw, pw)──► user_data_rules::recover ──► Session::log_in
//!                                                    ──► the new recovery code, once
//! the frame   ──login_log_out()──────────► Session::log_out
//! ```

use serde::Serialize;
use tauri::State;
use zeroize::Zeroizing;

use crate::objects::user::user_data::user_data_rules::{installed_user, log_in, recover};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::recovery_code::RecoveryCodeView;

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
/// Input: `session`, the session of the window.
/// Output: the state (`{"installed": true, "loggedIn": "Kamil"}`, `"loggedIn": null` at the login);
/// an error of the system database or the session.
///
/// Called by: `ui/src/lib/api/program_start.ts`: `loginGet`, from `ui/src/App.svelte`, when the window opens and
/// after every change.
/// Calls: `objects/user/user_data/user_data_rules.rs`: `installed_user`; `panels/session.rs`: `Session::user_name`.
#[tauri::command(async)]
pub fn login_get(session: State<'_, Session>) -> Result<ProgramView, ViewError> {
    let installed = installed_user(session.data_folder())?.is_some();
    let logged_in = session.user_name()?;

    Ok(ProgramView { installed, logged_in })
}

/// Logs the user in with their name and password, and keeps their open
/// database in the session. Run apart from the window (`async`): unwrapping
/// the key takes a moment by design (Argon2id). The password is wiped from
/// memory when the command ends.
///
/// Input: `session`, the session of the window; `name`, `password`, as typed.
/// Output: nothing; `ViewError::Account` for a wrong name or password, an
/// error of the databases or the session.
///
/// Called by: `ui/src/lib/api/program_start.ts`: `loginLogIn`, from `ui/src/views/program_start/LoginForm.svelte`,
/// "Log in".
/// Calls: `objects/user/user_data/user_data_rules.rs`: `log_in`; `panels/session.rs`: `Session::log_in`.
#[tauri::command(async)]
pub fn login_log_in(session: State<'_, Session>, name: String, password: String) -> Result<(), ViewError> {
    let password = Zeroizing::new(password);
    let (user, database) = log_in(session.data_folder(), &name, &password)?;

    session.log_in(user, database)?;

    Ok(())
}

/// The recovery of a forgotten password: the recovery code opens the
/// database of the user `name`, the new password and a new recovery code
/// replace the old ones, and the user is logged in. Gives the new code, to
/// be shown once. Run apart from the window (`async`): the new password is
/// wrapped by Argon2id. The passwords and the code are wiped from memory
/// when the command ends.
///
/// Input: `session`, the session of the window; `name`, the name typed;
/// `code`, the recovery code typed; `password`, `password_repeated`, the new
/// password typed twice.
/// Output: the new recovery code (`{"code": "ABCDE-..."}`); `ViewError::Account`
/// for a wrong name or code or a password refused, an error of the session.
///
/// Called by: `ui/src/lib/api/program_start.ts`: `loginRecover`, from
/// `ui/src/views/program_start/LoginForm.svelte`, "Set the new password".
/// Calls: `objects/user/user_data/user_data_rules.rs`: `recover`; `panels/session.rs`: `Session::log_in`;
/// `panels/views/recovery_code.rs`: `RecoveryCodeView::new`.
#[tauri::command(async)]
pub fn login_recover(
    session: State<'_, Session>,
    name: String,
    code: String,
    password: String,
    password_repeated: String,
) -> Result<RecoveryCodeView, ViewError> {
    let code = Zeroizing::new(code);
    let password = Zeroizing::new(password);
    let password_repeated = Zeroizing::new(password_repeated);
    let recovered = recover(session.data_folder(), &name, &code, &password, &password_repeated)?;
    let view = RecoveryCodeView::new(&recovered.code);

    session.log_in(recovered.user, recovered.database)?;

    Ok(view)
}

/// Logs the user out: their database is closed.
///
/// Called by: `ui/src/lib/api/program_start.ts`: `loginLogOut`, from the frame of the window (`ui/src/App.svelte`),
/// "Log out".
/// Calls: `panels/session.rs`: `Session::log_out`.
#[tauri::command(async)]
pub fn login_log_out(session: State<'_, Session>) -> Result<(), ViewError> {
    session.log_out()?;

    Ok(())
}
