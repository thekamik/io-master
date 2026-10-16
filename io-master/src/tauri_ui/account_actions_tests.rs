//! Unit tests of `account_actions.rs`.

use super::*;

use crate::tauri_ui::test_support::{PASSWORD, first_start, installed, logged_in};

#[tokio::test]
async fn the_setup_installs_and_logs_the_user_in() {
    let test = first_start();

    let view = set_up(
        &test.state,
        " anna ".to_owned(),
        PASSWORD.to_owned(),
        PASSWORD.to_owned(),
        "workstation".to_owned(),
        true,
    )
    .await
    .unwrap();

    assert!(view.installed);
    assert_eq!(view.role, Some("workstation"));
    assert_eq!(view.user.as_deref(), Some("anna"));
    assert_eq!(view.pages.len(), 5);
    assert!(!view.version.is_empty());
    assert_eq!(view.warnings.len(), 2);

    // Only once.
    let again = set_up(&test.state, "jan".into(), PASSWORD.into(), PASSWORD.into(), "master".into(), true).await;
    assert!(matches!(again, Err(UiError::AlreadyInstalled)));
}

#[tokio::test]
async fn an_incomplete_setup_installs_nothing() {
    let test = first_start();
    let cases = [
        ("anna", PASSWORD, "other password", "workstation", true),
        ("anna", PASSWORD, PASSWORD, "", true),
        ("anna", PASSWORD, PASSWORD, "workstation", false),
        ("anna", "short", "short", "workstation", true),
        ("", PASSWORD, PASSWORD, "workstation", true),
    ];

    for (name, password, repeat, role, confirmed) in cases {
        let result = set_up(&test.state, name.into(), password.into(), repeat.into(), role.into(), confirmed).await;

        assert!(matches!(result, Err(UiError::Invalid(_))), "{name} {role} {confirmed}");
    }

    assert!(test.state.running().is_none());
    assert!(!app_view(&test.state).installed);
}

#[tokio::test]
async fn a_wrong_password_and_an_unknown_name_are_refused_alike() {
    let test = installed();

    let wrong = log_in(&test.state, "anna".into(), "wrong password 1".into()).await;
    let unknown = log_in(&test.state, "nobody".into(), PASSWORD.into()).await;

    assert!(matches!(wrong, Err(UiError::WrongCredentials)));
    assert!(matches!(unknown, Err(UiError::WrongCredentials)));
    assert_eq!(app_view(&test.state).user, None);
}

#[tokio::test]
async fn logout_ends_the_session() {
    let test = logged_in().await;

    assert_eq!(app_view(&test.state).user.as_deref(), Some("anna"));
    assert_eq!(log_out(&test.state).user, None);
    assert!(matches!(test.state.session(), Err(UiError::NotLoggedIn)));
}

#[tokio::test]
async fn the_password_is_changed_with_the_current_one() {
    let test = logged_in().await;
    let new = "another long password";

    let wrong = change_password(&test.state, "not the password".into(), new.into(), new.into()).await;
    assert!(matches!(wrong, Err(UiError::Invalid(reason)) if reason.contains("current password")));

    let differ = change_password(&test.state, PASSWORD.into(), new.into(), "different".into()).await;
    assert!(matches!(differ, Err(UiError::Invalid(_))));

    change_password(&test.state, PASSWORD.into(), new.into(), new.into()).await.unwrap();

    log_out(&test.state);
    assert!(log_in(&test.state, "anna".into(), PASSWORD.into()).await.is_err());
    assert!(log_in(&test.state, "anna".into(), new.into()).await.is_ok());
}
