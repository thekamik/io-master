//! Unit tests of `state.rs`.

use super::*;

use crate::db::encryption::database_key::DatabaseKey;
use crate::db::user::database as user_db;
use crate::installation::session::Session;
use crate::security::random::fill_random;
use crate::test_support::TempDir;

fn state() -> AppState {
    AppState::new(PathBuf::from("/nonexistent"), Opened::FirstRun)
}

fn account() -> Account {
    Account { id: 1, name: "anna".to_owned() }
}

#[test]
fn a_session_starts_and_ends() {
    let state = state();

    assert!(matches!(state.session(), Err(AppError::NotLoggedIn)));

    // The database of the account, and its session.
    let directory = TempDir::new();
    let key = DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap();
    let again = DatabaseKey::fill_with(|bytes| {
        bytes.copy_from_slice(key.expose());
        Ok::<(), ()>(())
    })
    .unwrap();
    user_db::create(directory.path(), 1, &key).unwrap();

    state.log_in(Session::open(directory.path(), account(), again).unwrap());
    assert_eq!(state.current_user(), Some(account()));
    assert_eq!(state.session().unwrap().account, account());

    state.log_out();
    assert_eq!(state.current_user(), None);
}

#[test]
fn nothing_is_installed_at_a_first_start() {
    assert!(matches!(state().installed(), Err(AppError::NotInstalled)));
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
