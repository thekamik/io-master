//! The command of the password form.

use tauri::State;
use zeroize::Zeroizing;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::installation::accounts::{self, AccountError};

/// Changes the password of the logged-in evaluator. The current
/// password is needed; the two new ones must agree.
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn password_form_change(
    state: State<'_, AppState>,
    current: String,
    password: String,
    password_repeat: String,
) -> Result<(), AppError> {
    // The passwords are wiped from memory when they are dropped.
    let current = Zeroizing::new(current);
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    ensure!(*password == *password_repeat, AppError::Invalid("the two new passwords differ".to_owned()));

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
        Err(AccountError::WrongCredentials) => Err(AppError::Invalid("the current password is wrong".to_owned())),
        Err(other) => Err(other.into()),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `password_form/commands.rs`, through the IPC of Tauri.

    use serde_json::json;

    use crate::app::test_support::PASSWORD;
    use crate::views::test_support::logged_in_window;

    #[test]
    fn the_password_is_changed_with_the_current_one() {
        let window = logged_in_window();
        let new = "another long password";
        let change = |current: &str, password: &str, repeat: &str| {
            window.call(
                "password_form_change",
                json!({ "current": current, "password": password, "passwordRepeat": repeat }),
            )
        };

        let wrong = change("not the password", new, new).unwrap_err();
        assert_eq!(wrong["kind"], "invalid");
        assert!(wrong["message"].as_str().unwrap().contains("current password"));

        assert_eq!(change(PASSWORD, new, "different").unwrap_err()["kind"], "invalid");

        change(PASSWORD, new, new).unwrap();

        // The new password opens the database; the old one no longer does.
        window.call("user_menu_log_out", json!({})).unwrap();
        assert!(window.call("login_form_log_in", json!({ "name": "anna", "password": PASSWORD })).is_err());
        window.call("login_form_log_in", json!({ "name": "anna", "password": new })).unwrap();
    }
}
