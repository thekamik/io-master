//! The command of the login form.

use tauri::State;
use zeroize::Zeroizing;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::installation::{accounts, session::Session};

/// Logs in with the name and the password. A wrong password and an
/// unknown name give the same error (`AppError::WrongCredentials`), so
/// that the window does not tell which names exist.
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn login_form_log_in(state: State<'_, AppState>, name: String, password: String) -> Result<(), AppError> {
    let password = Zeroizing::new(password);
    let running = state.installed()?;

    let (account, key) =
        accounts::log_in(&running.system_db, running.device_secret.as_ref(), &state.data_dir, name.trim(), &password)?;

    state.log_in(Session::open(&state.data_dir, account, key)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests of `login_form/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::app::test_support::PASSWORD;
    use crate::views::test_support::{first_start_window, logged_out_window};

    #[test]
    fn the_right_password_logs_in() {
        let window = logged_out_window();

        window.call("login_form_log_in", json!({ "name": " anna ", "password": PASSWORD })).unwrap();

        assert_eq!(window.call("program_get", json!({})).unwrap()["user"], "anna");
    }

    #[test]
    fn a_wrong_password_and_an_unknown_name_are_refused_alike() {
        let window = logged_out_window();

        let wrong = window.call("login_form_log_in", json!({ "name": "anna", "password": "wrong password 1" }));
        let unknown = window.call("login_form_log_in", json!({ "name": "nobody", "password": PASSWORD }));

        assert_eq!(wrong.unwrap_err(), unknown.unwrap_err());
        assert_eq!(window.call("program_get", json!({})).unwrap()["user"], json!(null));
    }

    #[test]
    fn nobody_logs_in_before_the_setup() {
        let window = first_start_window();

        let error = window.call("login_form_log_in", json!({ "name": "anna", "password": PASSWORD })).unwrap_err();

        assert_eq!(error["kind"], "not_installed");
    }
}
