//! The password of the logged-in evaluator (the tab User of Settings;
//! moved to `views/settings/` with that view). The first start, the
//! login and the logout are in `views/start/` and `views/shell/`.

use zeroize::Zeroizing;

use crate::installation::accounts;

use crate::tauri_ui::blocking::blocking;
use crate::tauri_ui::error::UiError;
use crate::tauri_ui::state::UiState;

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
#[path = "account_tests.rs"]
mod tests;
