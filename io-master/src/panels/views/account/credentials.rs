//! The panel Account, as the page calls it (`ui/src/views/account/`): the
//! change of the password and a new recovery code, for the user logged in.
//! Each gives a new recovery code, shown once; the old one stops working.
//!
//! ```text
//! PasswordForm      ──account_change_password(current, pw, pw)──► user_data_rules::change_password
//! RecoveryCodeForm  ──account_new_recovery_code(pw)─────────────► user_data_rules::new_recovery_code
//!                                                                  ──► the new recovery code, once
//! ```

use tauri::State;
use zeroize::Zeroizing;

use crate::objects::user::user_data::user_data_rules::{change_password, new_recovery_code};
use crate::panels::session::Session;
use crate::panels::views::error::ViewError;
use crate::panels::views::recovery_code::RecoveryCodeView;

/// Changes the password of the user logged in, with their present password;
/// the new one is given twice. Gives the new recovery code, to be shown
/// once. Run apart from the window (`async`): the passwords go through
/// Argon2id. The passwords are wiped from memory when the command ends.
///
/// Called by: `PasswordForm.svelte`, "Change the password".
/// Calls: `Session::user_id`; `objects/user/user_data/user_data_rules.rs`:
/// `change_password`; `panels/views/recovery_code.rs`: `RecoveryCodeView::new`.
#[tauri::command(async)]
pub fn account_change_password(
    session: State<'_, Session>,
    current: String,
    password: String,
    password_repeated: String,
) -> Result<RecoveryCodeView, ViewError> {
    let current = Zeroizing::new(current);
    let password = Zeroizing::new(password);
    let password_repeated = Zeroizing::new(password_repeated);
    let user = session.user_id()?.ok_or(ViewError::NotLoggedIn)?;
    let code = change_password(session.data_folder(), user, &current, &password, &password_repeated)?;

    Ok(RecoveryCodeView::new(&code))
}

/// Makes a new recovery code for the user logged in, with their password,
/// when the old one is lost or may have been seen. Gives the new code, to
/// be shown once. Run apart from the window (`async`): the password goes
/// through Argon2id. The password is wiped from memory when the command
/// ends.
///
/// Called by: `RecoveryCodeForm.svelte`, "Make a new recovery code".
/// Calls: `Session::user_id`; `objects/user/user_data/user_data_rules.rs`:
/// `new_recovery_code`; `panels/views/recovery_code.rs`: `RecoveryCodeView::new`.
#[tauri::command(async)]
pub fn account_new_recovery_code(session: State<'_, Session>, password: String) -> Result<RecoveryCodeView, ViewError> {
    let password = Zeroizing::new(password);
    let user = session.user_id()?.ok_or(ViewError::NotLoggedIn)?;
    let code = new_recovery_code(session.data_folder(), user, &password)?;

    Ok(RecoveryCodeView::new(&code))
}
