//! Unit tests of `connection_check/commands.rs`, with a fake AI service.
//! The fake service runs on a runtime of its own, whose threads serve it
//! while the test waits for the command.

use serde_json::{Value, json};
use tokio::runtime::Runtime;

use crate::ai::api::connections;
use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
use crate::model::api_key::ApiKey;
use crate::views::test_support::{TestWindow, logged_in_window};

const API_KEY: &str = "sk-secret-of-anna";

/// A window with a connection to `service`, and its id.
fn with_connection(service: &FakeService) -> (TestWindow, i64) {
    let window = logged_in_window();
    let id = connections::create(&window.session().db(), &completions(service)).unwrap().id;

    (window, id)
}

/// The report of the check of connection `id`.
fn check(window: &TestWindow, id: i64) -> Value {
    window.call("connection_check_run", json!({ "id": id })).unwrap()
}

#[test]
fn a_check_reports_with_the_key_masked() {
    let runtime = Runtime::new().unwrap();
    let service = runtime.block_on(FakeService::start(FakeAnswer::Stream(completions_events(&["O", "K"]))));
    let (window, id) = with_connection(&service);
    connections::set_key(&window.session().db(), id, &ApiKey::new(API_KEY.to_owned()).unwrap()).unwrap();

    let report = check(&window, id);

    assert_eq!(report["works"], true, "{}", report["outcome"]);
    assert_eq!(report["text"], "OK");
    assert_eq!(report["status"], 200);
    assert!(!report.to_string().contains(API_KEY));
    assert_eq!(service.received()[0].header("authorization"), Some(format!("Bearer {API_KEY}").as_str()));
}

#[test]
fn a_check_without_the_needed_key_explains_it() {
    let runtime = Runtime::new().unwrap();
    let service = runtime.block_on(FakeService::start(FakeAnswer::Stream(completions_events(&["OK"]))));
    let (window, id) = with_connection(&service);

    let report = check(&window, id);

    assert_eq!(report["works"], false);
    assert!(report["outcome"].as_str().unwrap().contains("API key"), "{}", report["outcome"]);
    assert!(service.received().is_empty());
}
