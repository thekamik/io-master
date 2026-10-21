//! Unit tests of `commands/account.rs`.

use super::*;

use crate::tauri_ui::test_support::{PASSWORD, log_in, logged_in};

#[tokio::test]
async fn the_password_is_changed_with_the_current_one() {
    let test = logged_in().await;
    let new = "another long password";

    let wrong = change_password(&test.state, "not the password".into(), new.into(), new.into()).await;
    assert!(matches!(wrong, Err(UiError::Invalid(reason)) if reason.contains("current password")));

    let differ = change_password(&test.state, PASSWORD.into(), new.into(), "different".into()).await;
    assert!(matches!(differ, Err(UiError::Invalid(_))));

    change_password(&test.state, PASSWORD.into(), new.into(), new.into()).await.unwrap();

    // The new password opens the database; the old one no longer does.
    test.state.log_out();
    log_in(&test.state, new);
    assert!(test.state.session().is_ok());
}
