//! The entry of the view Users: the account of the evaluator, its session
//! and the encryption of its database, in one place (step 47 of
//! docs/todo.md), so that the way of the keys can be followed: the setup
//! that makes the user, the login, the change of the password, the recovery
//! code (step 49), the logout (page: `ui/src/views/users/`, its commands in
//! `ui/src/lib/api/users.ts`). Those that make a recovery code give it back
//! to be shown once (`components/recovery_code/`); the page then asks for
//! the state of the program again (`views/shell/shell.rs`, `program_get`).
//!
//! The keys (`docs/architecture.md`, section 5.2): the key of the database
//! is random and is stored only encrypted, with an unlocking key made from
//! the password by Argon2id (`db/encryption/keys.rs`), in the system
//! database. It is opened at the login, opens the database, and is wiped;
//! the open database (SQLCipher keeps its key inside the connection) lives
//! in the session until the logout. The way of the key in every operation:
//! `docs/architecture.md`, section 12.

use tauri::{AppHandle, Runtime, State};
use zeroize::Zeroizing;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::components::recovery_code::view::RecoveryCodeView;
use crate::installation::accounts::{self, AccountError};
use crate::installation::lifecycle;
use crate::installation::session::Session;
use crate::model::recovery_code::RecoveryCode;

// ── Before anybody is logged in ───────────────────────────────────────

/// The first start: makes the installation and its one user `name`, logs
/// that user in, and gives back the first recovery code, made automatically
/// (user, 2026-10-24), to be shown once. Refused when the program is
/// installed already or when the passwords differ.
///
/// Called by the page: `SetupForm.svelte`, "Install" (`users.setupFormInstall(...)`).
/// Calls: `lifecycle::install` (the system database, and the
/// account: a new random key of the database, the database made with it,
/// the key stored encrypted with the password), then `accounts::log_in`,
/// `Session::open`, `AppState::install` and `AppState::log_in`, as the login.
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
) -> Result<RecoveryCodeView, AppError> {
    // The passwords are wiped from memory when they are dropped.
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    ensure!(state.running().is_none(), AppError::AlreadyInstalled);
    ensure!(*password == *password_repeat, AppError::Invalid("the two passwords differ".to_owned()));

    let name = name.trim();
    let (running, _, code) = lifecycle::install(&state.data_dir, name, &password)?;
    let (account, key) = accounts::log_in(&running.system_db, &state.data_dir, name, &password)?;
    let session = Session::open(&state.data_dir, account, key)?;

    state.install(running);
    state.log_in(session);

    Ok(RecoveryCodeView::new(&code))
}

/// Logs in with the name and the password. A wrong password and an
/// unknown name give the same error (`AppError::WrongCredentials`), so
/// that the window does not tell which names exist.
///
/// Called by the page: `LoginForm.svelte`, "Log in" (`users.loginFormLogIn(...)`).
/// Calls: `accounts::log_in` (the password through Argon2id gives the
/// unlocking key, which decrypts the key of the database; a wrong password
/// fails here), `Session::open` (the database opened with the key, the key
/// wiped), `AppState::log_in` (the session kept until the logout).
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn login_form_log_in(state: State<'_, AppState>, name: String, password: String) -> Result<(), AppError> {
    let password = Zeroizing::new(password);
    let running = state.installed()?;

    let (account, key) = accounts::log_in(&running.system_db, &state.data_dir, name.trim(), &password)?;

    state.log_in(Session::open(&state.data_dir, account, key)?);

    Ok(())
}

/// The recovery of a forgotten password: the recovery code (as typed)
/// opens the key of the database, the new password and a new recovery code
/// replace the old ones, the evaluator is logged in, and the new code is
/// given back to be shown once. A wrong code and an unknown name give the
/// same error as a wrong password (`AppError::WrongCredentials`).
///
/// Called by the page: `LoginForm.svelte`, "I do not remember my password"
/// (`users.loginFormRecover(...)`).
/// Calls: `RecoveryCode::parse` (the code checked at its entry),
/// `accounts::recover` (the code through HKDF opens the copy of the key made
/// for it; the key wrapped anew with the new password and a new code),
/// `Session::open`, `AppState::log_in`.
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn login_form_recover(
    state: State<'_, AppState>,
    name: String,
    code: String,
    password: String,
    password_repeat: String,
) -> Result<RecoveryCodeView, AppError> {
    // The code and the passwords are wiped from memory when they are dropped.
    let code = Zeroizing::new(code);
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    ensure!(*password == *password_repeat, AppError::Invalid("the two passwords differ".to_owned()));

    let code = RecoveryCode::parse(&code).map_err(|error| AppError::Invalid(error.to_string()))?;
    let running = state.installed()?;
    let (account, key, new_code) =
        accounts::recover(&running.system_db, &state.data_dir, name.trim(), &code, &password)?;

    state.log_in(Session::open(&state.data_dir, account, key)?);

    Ok(RecoveryCodeView::new(&new_code))
}

// ── The account of the logged-in evaluator ───────────────────────────

/// Changes the password of the logged-in evaluator. The current
/// password is needed; the two new ones must agree. A new recovery code is
/// made and given back to be shown once; the old one stops working (user,
/// 2026-10-24).
///
/// Called by the page: `PasswordForm.svelte` of the page User, "Change
/// password" (`users.passwordFormChange(...)`).
/// Calls: `accounts::change_password` (the current password decrypts the key
/// of the database, which is encrypted again with an unlocking key made
/// from the new password and with a new recovery code, both stored; the
/// database itself is not touched).
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
) -> Result<RecoveryCodeView, AppError> {
    // The passwords are wiped from memory when they are dropped.
    let current = Zeroizing::new(current);
    let password = Zeroizing::new(password);
    let password_repeat = Zeroizing::new(password_repeat);

    ensure!(*password == *password_repeat, AppError::Invalid("the two new passwords differ".to_owned()));

    let running = state.installed()?;
    let session = state.session()?;

    let changed = accounts::change_password(&running.system_db, &session.account.name, &current, &password);

    match changed {
        Ok(code) => Ok(RecoveryCodeView::new(&code)),
        // Here only the current password can be wrong.
        Err(AccountError::WrongCredentials) => Err(AppError::Invalid("the current password is wrong".to_owned())),
        Err(other) => Err(other.into()),
    }
}

/// Makes a new recovery code for the logged-in evaluator; the password is
/// needed (user, 2026-10-24) and the old code stops working. The new code
/// is given back to be shown once.
///
/// Called by the page: `RecoveryCodeForm.svelte` of the page User, "Make a
/// new recovery code" (`users.recoveryCodeFormReplace(...)`).
/// Calls: `accounts::replace_recovery_code` (the password opens the key,
/// which is wrapped with a new code; the copy of the password stays).
///
/// `async`: on a thread of the runtime of Tauri, not of the window,
/// which keeps answering while Argon2id works.
// Tauri hands a command its state and its arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn recovery_code_form_replace(state: State<'_, AppState>, password: String) -> Result<RecoveryCodeView, AppError> {
    let password = Zeroizing::new(password);
    let running = state.installed()?;
    let session = state.session()?;

    match accounts::replace_recovery_code(&running.system_db, &session.account.name, &password) {
        Ok(code) => Ok(RecoveryCodeView::new(&code)),
        // Here only the password can be wrong.
        Err(AccountError::WrongCredentials) => Err(AppError::Invalid("the password is wrong".to_owned())),
        Err(other) => Err(other.into()),
    }
}

// ── The end of the session ───────────────────────────────────────────

/// Logs out: ends the session of the evaluator.
///
/// Called by the page: `UserMenu.svelte`, "Log out" (`users.userMenuLogOut()`).
/// Calls: `AppState::log_out` (a reply being written and a long work are
/// stopped; the database, and the key SQLCipher keeps in it, close when the
/// last holder of the session lets it go).
// Tauri hands a command its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn user_menu_log_out(state: State<'_, AppState>) {
    state.log_out();
}

/// Logs out and closes the program.
///
/// Called by the page: `UserMenu.svelte`, "Log out and close"
/// (`users.userMenuClose()`).
/// Calls: `AppState::log_out` (as "Log out"), then `AppHandle::exit`, whose
/// event of the end ends the session once more on its way out (`main.rs`):
/// the cross of the window takes that way alone.
// Tauri hands a command its handle and its state by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
pub fn user_menu_close<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) {
    state.log_out();
    app.exit(0);
}

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Users, called as the page calls
    //! them: by name, through the IPC of Tauri.

    use serde_json::{Value, json};

    use crate::app::test_support::PASSWORD;
    use crate::views::test_support::{first_start_window, logged_in_window, logged_out_window};

    /// The form of the setup.
    fn form(name: &str, password: &str, repeat: &str) -> Value {
        json!({ "name": name, "password": password, "passwordRepeat": repeat })
    }

    #[test]
    fn the_setup_installs_and_logs_the_user_in_once() {
        let window = first_start_window();

        let code = window.call("setup_form_install", form(" anna ", PASSWORD, PASSWORD)).unwrap();

        // The first recovery code comes with the setup: seven groups of five.
        assert_eq!(code["code"].as_str().unwrap().split('-').count(), 7);

        let program = window.call("program_get", json!({})).unwrap();
        assert_eq!(program["installed"], true);
        assert_eq!(program["user"], "anna");

        let again = window.call("setup_form_install", form("jan", PASSWORD, PASSWORD)).unwrap_err();
        assert_eq!(again["kind"], "already_installed");
    }

    #[test]
    fn an_incomplete_setup_installs_nothing() {
        let window = first_start_window();
        let cases =
            [form("anna", PASSWORD, "other password"), form("anna", "short", "short"), form("", PASSWORD, PASSWORD)];

        for case in cases {
            let error = window.call("setup_form_install", case.clone()).unwrap_err();

            assert_eq!(error["kind"], "invalid", "{case}");
        }

        assert_eq!(window.call("program_get", json!({})).unwrap()["installed"], false);
    }

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

        let code = change(PASSWORD, new, new).unwrap();
        assert_eq!(code["code"].as_str().unwrap().len(), 41);

        // The new password opens the database; the old one no longer does.
        window.call("user_menu_log_out", json!({})).unwrap();
        assert!(window.call("login_form_log_in", json!({ "name": "anna", "password": PASSWORD })).is_err());
        window.call("login_form_log_in", json!({ "name": "anna", "password": new })).unwrap();
    }

    #[test]
    fn the_logout_ends_the_session() {
        let window = logged_in_window();

        window.call("user_menu_log_out", json!({})).unwrap();

        assert_eq!(window.call("program_get", json!({})).unwrap()["user"], json!(null));
        assert_eq!(window.call("chat_list_get", json!({})).unwrap_err()["kind"], "not_logged_in");
    }

    /// The recovery of the password of "anna" with `code`, as the login form
    /// sends it.
    fn recover(window: &crate::views::test_support::TestWindow, code: &Value, password: &str) -> Result<Value, Value> {
        window.call(
            "login_form_recover",
            json!({ "name": "anna", "code": code, "password": password, "passwordRepeat": password }),
        )
    }

    #[test]
    fn the_code_of_the_setup_recovers_the_password_once() {
        let window = first_start_window();
        let first = window.call("setup_form_install", form("anna", PASSWORD, PASSWORD)).unwrap()["code"].clone();
        window.call("user_menu_log_out", json!({})).unwrap();

        // Typed in small letters with spaces: still the code.
        let typed = json!(first.as_str().unwrap().to_lowercase().replace('-', " "));
        let second = recover(&window, &typed, "a new long password").unwrap()["code"].clone();

        assert_eq!(window.call("program_get", json!({})).unwrap()["user"], "anna");
        assert_ne!(first, second);

        // The used code no longer works; the new password does.
        window.call("user_menu_log_out", json!({})).unwrap();
        assert_eq!(recover(&window, &first, "another long password").unwrap_err()["kind"], "wrong_credentials");
        window.call("login_form_log_in", json!({ "name": "anna", "password": "a new long password" })).unwrap();
    }

    #[test]
    fn a_wrong_or_mistyped_code_recovers_nothing() {
        let window = first_start_window();
        let code = window.call("setup_form_install", form("anna", PASSWORD, PASSWORD)).unwrap()["code"].clone();
        window.call("user_menu_log_out", json!({})).unwrap();

        let text = code.as_str().unwrap();
        let other = if text.starts_with('A') { 'B' } else { 'A' };
        let mistyped = json!(format!("{other}{}", &text[1..]));

        assert_eq!(recover(&window, &mistyped, "a new long password").unwrap_err()["kind"], "invalid");
        assert_eq!(recover(&window, &json!("not a code"), "a new long password").unwrap_err()["kind"], "invalid");
        assert_eq!(window.call("program_get", json!({})).unwrap()["user"], json!(null));
        window.call("login_form_log_in", json!({ "name": "anna", "password": PASSWORD })).unwrap();
    }

    #[test]
    fn a_new_code_needs_the_password_and_ends_the_old_one() {
        let window = first_start_window();
        let first = window.call("setup_form_install", form("anna", PASSWORD, PASSWORD)).unwrap()["code"].clone();

        let wrong = window.call("recovery_code_form_replace", json!({ "password": "not the password" })).unwrap_err();
        assert_eq!(wrong["kind"], "invalid");

        let second =
            window.call("recovery_code_form_replace", json!({ "password": PASSWORD })).unwrap()["code"].clone();
        assert_ne!(first, second);

        window.call("user_menu_log_out", json!({})).unwrap();
        assert_eq!(recover(&window, &first, "a new long password").unwrap_err()["kind"], "wrong_credentials");
        recover(&window, &second, "a new long password").unwrap();
    }

    // `user_menu_close` is not called here: the mock runtime of Tauri does
    // not implement the exit of the program. Its logout is the one tested
    // above; the closing itself is tested in the window (docs/handover.md).
}
