//! Unit tests of `oscal_documents/commands.rs`. The import and the export
//! open dialogs of the system, so their files are tested in
//! `file_tests.rs`; here the list, the deletion and the name of a file.

use serde_json::json;

use super::safe_name;
use crate::views::plans::oscal_documents::file;
use crate::views::test_support::logged_in_window;

#[test]
fn a_document_is_listed_and_deleted() {
    let window = logged_in_window();
    let catalog = include_str!("../../../../oscal/examples/basic-catalog.json");
    let uuid = file::import(&window.session().db(), catalog).unwrap().uuid;

    let listed = window.call("oscal_documents_get", json!({})).unwrap();
    assert_eq!(listed[0]["uuid"], uuid);
    assert_eq!(listed[0]["model"], "catalog");

    window.call("oscal_documents_delete", json!({ "uuid": uuid })).unwrap();
    assert_eq!(window.call("oscal_documents_get", json!({})).unwrap(), json!([]));
    assert_eq!(window.call("oscal_documents_delete", json!({ "uuid": uuid })).unwrap_err()["kind"], "invalid");
}

#[test]
fn a_name_for_a_file_keeps_only_safe_characters() {
    assert_eq!(safe_name(" CC Part 2: SFR / v1.0 "), "CC Part 2_ SFR _ v1.0");
    assert_eq!(safe_name("../../etc"), ".._.._etc");
    assert_eq!(safe_name("   "), "oscal");
}
