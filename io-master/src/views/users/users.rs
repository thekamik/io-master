//! The entry of the view Users: the account of the evaluator, its session
//! and the encryption of its database, in one place (step 47 of
//! docs/todo.md), so that the way of the keys can be followed: the setup
//! that makes the user, the login, the change of the password, the logout
//! (page: `ui/src/views/users/`, its commands in
//! `ui/src/lib/api/users.ts`). None gives data back: the page then asks for
//! the state of the program again (`views/shell/shell.rs`, `program_get`).
//!
//! The keys (`docs/architecture.md`, section 5.2): the key of the database
//! is random and is stored only encrypted, with an unlocking key made from
//! the password by Argon2id (`db/encryption/keys.rs`), in the system
//! database. It is opened at the login, opens the database, and is wiped;
//! the open database (SQLCipher keeps its key inside the connection) lives
//! in the session until the logout. The way of the key in every operation:
//! `docs/architecture.md`, section 12.

use tauri::State;
use zeroize::Zeroizing;

use crate::app::error::AppError;
use crate::app::state::AppState;
use crate::installation::accounts::{self, AccountError};
use crate::installation::lifecycle;
use crate::installation::session::Session;
use crate::model::installation::Role;

// ── Before anybody is logged in ───────────────────────────────────────

/// The first start: makes the installation with `role` (`"workstation"`
/// or `"master"`) and its user `name`, and logs that user in. Refused
/// when the program is installed already, when the passwords differ,
/// when no role is chosen or its finality is not confirmed.
///
/// Called by the page: `SetupForm.svelte`, "Install" (`users.setupFormInstall(...)`).
/// Calls: `lifecycle::install` (the system database with the role, and the
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

    let (account, key) =
        accounts::log_in(&running.system_db, running.device_secret.as_ref(), &state.data_dir, name.trim(), &password)?;

    state.log_in(Session::open(&state.data_dir, account, key)?);

    Ok(())
}

// ── The account of the logged-in evaluator ───────────────────────────

/// Changes the password of the logged-in evaluator. The current
/// password is needed; the two new ones must agree.
///
/// Called by the page: `PasswordForm.svelte` of the page User, "Change
/// password" (`users.passwordFormChange(...)`).
/// Calls: `accounts::change_password` (the current password decrypts the key
/// of the database, which is encrypted again with an unlocking key made
/// from the new password and stored; the database itself is not touched).
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

#[cfg(test)]
mod tests {
    //! Tests of the commands of the view Users, called as the page calls
    //! them: by name, through the IPC of Tauri.

    use serde_json::{Value, json};

    use crate::app::test_support::PASSWORD;
    use crate::views::test_support::{first_start_window, logged_in_window, logged_out_window};

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

        change(PASSWORD, new, new).unwrap();

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
        assert_eq!(window.call("ports_table_scan", json!({})).unwrap_err()["kind"], "not_logged_in");
    }
}
