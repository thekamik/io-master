//! Help for the tests of the desktop interface: a state with a real
//! installation (databases, accounts) in a temporary directory.

use crate::installation::lifecycle::{self, Opened};
use crate::test_support::TempDir;

use super::state::AppState;
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
    pub state: AppState,
}

/// A state at the first start.
pub fn first_start() -> TestState {
    let directory = TempDir::new();
    let state = AppState::new(directory.path().to_owned(), Opened::FirstRun);

    TestState { directory, state }
}

/// The program installed by "anna", nobody logged in yet.
pub fn installed() -> TestState {
    let directory = TempDir::new();
    let (running, _) = lifecycle::install(directory.path(), "anna", PASSWORD).unwrap();
    let state = AppState::new(directory.path().to_owned(), Opened::Installed(running));

    TestState { directory, state }
}

/// The program with "anna" logged in.
pub async fn logged_in() -> TestState {
    let test = installed();
    log_in(&test.state, PASSWORD);

    test
}

/// Logs "anna" in with `password`, as the login form does.
pub fn log_in(state: &AppState, password: &str) {
    let running = state.installed().unwrap();
    let (account, key) = accounts::log_in(&running.system_db, &state.data_dir, "anna", password).unwrap();

    state.log_in(Session::open(&state.data_dir, account, key).unwrap());
}
