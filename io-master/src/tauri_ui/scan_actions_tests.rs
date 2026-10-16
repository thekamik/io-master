//! Unit tests of `scan_actions.rs`. The scan reads the real ports of
//! the computer; the tests check the settings, not which ports exist.

use super::*;

use crate::tauri_ui::test_support::{installed, logged_in};

#[tokio::test]
async fn the_scan_uses_the_stored_settings() {
    let test = logged_in().await;

    let first = scan(&test.state).await.unwrap();
    assert!(first.settings.all_types);
    assert_eq!(first.settings.columns, ["path", "type", "device", "serial_number", "access"]);

    let saved = save_scan_settings(
        &test.state,
        false,
        vec!["ttyACM".into(), "".into()],
        vec!["path".into(), "not_a_column".into(), "driver".into()],
    )
    .await
    .unwrap();

    assert!(!saved.settings.all_types);
    assert_eq!(saved.settings.types, ["ttyACM"]);
    assert_eq!(saved.settings.columns, ["path", "driver"]);
    assert_eq!(saved.settings.summary, "ttyACM · 2 columns");

    if let Some(table) = &saved.table {
        assert_eq!(table.columns.len(), 2);
    }

    // Kept for the next scan.
    assert_eq!(scan(&test.state).await.unwrap().settings.types, ["ttyACM"]);
}

#[tokio::test]
async fn the_scan_needs_a_login() {
    let test = installed();

    assert!(matches!(scan(&test.state).await, Err(UiError::NotLoggedIn)));
}
