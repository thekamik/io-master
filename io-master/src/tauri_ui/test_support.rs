//! Help for the tests of the desktop interface: a state with a real
//! installation (databases, accounts) in a temporary directory.

use crate::installation::lifecycle::{self, Opened};
use crate::model::installation::Role;
use crate::test_support::TempDir;

use super::state::UiState;
use crate::installation::{accounts, session::Session};

/// The password of the user of the tests.
pub const PASSWORD: &str = "correct horse battery";

/// A state and its data directory; dropping it removes the
/// directory.
pub struct TestState {
    /// Not read by the tests; held so that the folder lives exactly
    /// as long as the state that uses it.
    #[allow(dead_code)]
    pub directory: TempDir,
    pub state: UiState,
}

/// A state at the first start.
pub fn first_start() -> TestState {
    let directory = TempDir::new();
    let state = UiState::new(directory.path().to_owned(), Opened::FirstRun);

    TestState { directory, state }
}

/// A workstation installed by "anna", nobody logged in yet.
pub fn installed() -> TestState {
    let directory = TempDir::new();
    let (running, _) = lifecycle::install(directory.path(), Role::Workstation, "anna", PASSWORD).unwrap();
    let state = UiState::new(directory.path().to_owned(), Opened::Installed(running));

    TestState { directory, state }
}

/// A workstation with "anna" logged in.
pub async fn logged_in() -> TestState {
    let test = installed();
    log_in(&test.state, PASSWORD);

    test
}

/// Logs "anna" in with `password`, as the login form does.
pub fn log_in(state: &UiState, password: &str) {
    let running = state.installed().unwrap();
    let (account, key) =
        accounts::log_in(&running.system_db, running.device_secret.as_ref(), &state.data_dir, "anna", password)
            .unwrap();

    state.log_in(Session::open(&state.data_dir, account, key).unwrap());
}

/// "anna" logged in, with one loaded document of three pages (the
/// small standard of `test_support/standards.rs`); returns the state and
/// the document.
pub async fn with_document() -> (TestState, i64) {
    let test = logged_in().await;
    let id = crate::test_support::standards::load(&test.state.session().unwrap().db(), b"%PDF-1.4 cc-part3");

    (test, id)
}
