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

#[cfg(test)]
mod tests {
    //! Unit tests of `setup_form/commands.rs`, through the IPC of Tauri.

    use serde_json::{Value, json};

    use crate::app::test_support::PASSWORD;
    use crate::views::test_support::first_start_window;

    /// The form of the setup.
    fn form(name: &str, password: &str, repeat: &str, role: &str, confirmed: bool) -> Value {
        json!({ "name": name, "password": password, "passwordRepeat": repeat, "role": role, "roleConfirmed": confirmed })
    }

    #[test]
    fn the_setup_installs_and_logs_the_user_in_once() {
        let window = first_start_window();

        window.call("setup_form_install", form(" anna ", PASSWORD, PASSWORD, "workstation", true)).unwrap();

        let program = window.call("program_get", json!({})).unwrap();
        assert_eq!(program["installed"], true);
        assert_eq!(program["role"], "workstation");
        assert_eq!(program["user"], "anna");

        let again = window.call("setup_form_install", form("jan", PASSWORD, PASSWORD, "master", true)).unwrap_err();
        assert_eq!(again["kind"], "already_installed");
    }

    #[test]
    fn an_incomplete_setup_installs_nothing() {
        let window = first_start_window();
        let cases = [
            form("anna", PASSWORD, "other password", "workstation", true),
            form("anna", PASSWORD, PASSWORD, "", true),
            form("anna", PASSWORD, PASSWORD, "workstation", false),
            form("anna", "short", "short", "workstation", true),
            form("", PASSWORD, PASSWORD, "workstation", true),
        ];

        for case in cases {
            let error = window.call("setup_form_install", case.clone()).unwrap_err();

            assert_eq!(error["kind"], "invalid", "{case}");
        }

        assert_eq!(window.call("program_get", json!({})).unwrap()["installed"], false);
    }
}
