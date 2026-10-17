//! Unit tests of `state.rs`.

use super::*;

use crate::db::encryption::database_key::DatabaseKey;
use crate::security::random::fill_random;

fn state() -> UiState {
    UiState::new(PathBuf::from("/nonexistent"), Opened::FirstRun)
}

fn account() -> Account {
    Account { id: 1, name: "anna".to_owned() }
}

#[test]
fn a_session_starts_and_ends() {
    let state = state();

    assert!(matches!(state.session(), Err(UiError::NotLoggedIn)));

    state.log_in(account(), DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap());
    assert_eq!(state.current_user(), Some(account()));
    assert_eq!(state.session().unwrap().account, account());

    state.log_out();
    assert_eq!(state.current_user(), None);
}

#[test]
fn nothing_is_installed_at_a_first_start() {
    assert!(matches!(state().installed(), Err(UiError::NotInstalled)));
}

#[test]
fn stop_and_a_new_reply_stop_the_reply_being_written() {
    let state = state();

    let first = state.start_reply();
    let second = state.start_reply();
    assert!(*first.borrow(), "a new reply stops the earlier one");
    assert!(!*second.borrow());

    state.stop_reply();
    assert!(*second.borrow());

    // Stopping without a reply does nothing.
    state.stop_reply();
}

#[test]
fn logging_out_stops_the_reply() {
    let state = state();
    let reply = state.start_reply();

    state.log_out();
    assert!(*reply.borrow());
}
