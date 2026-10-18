//! Unit tests of `oscal.rs`.

use super::*;

use crate::tauri_ui::test_support::logged_in;
use crate::test_support::TempDir;

#[tokio::test]
async fn a_file_of_oscal_is_imported_exported_and_deleted() {
    let test = logged_in().await;
    let directory = TempDir::new();
    let source = directory.path().join("catalog.json");
    std::fs::write(&source, include_str!("../../../oscal/examples/basic-catalog.json")).unwrap();

    let imported = import(&test.state, source).await.unwrap();
    assert_eq!(imported.model, "catalog");
    assert_eq!(list(&test.state).await.unwrap().len(), 1);

    let target = directory.path().join("out.json");
    export(&test.state, imported.uuid.clone(), &target).await.unwrap();
    let written = std::fs::read_to_string(&target).unwrap();
    assert!(written.starts_with("{\n  \"catalog\""), "{written}");

    delete(&test.state, imported.uuid.clone()).await.unwrap();
    assert!(list(&test.state).await.unwrap().is_empty());
    assert!(delete(&test.state, imported.uuid).await.is_err());
}
