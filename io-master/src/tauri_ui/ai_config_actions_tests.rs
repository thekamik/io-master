//! Unit tests of `ai_config_actions.rs`, with a fake AI service
//! (`ai/api/test_support.rs`).

use super::*;

use crate::ai::api::connection::Connection;
use crate::ai::api::test_support::{FakeAnswer, FakeService, completions, completions_events};
use crate::tauri_ui::test_support::{installed, logged_in};

const API_KEY: &str = "sk-secret-of-anna";

/// The form the page sends for a connection.
fn form(connection: Connection) -> ConnectionForm {
    ConnectionForm {
        name: connection.name,
        url: connection.url,
        model: connection.model,
        ca_certificate: connection.ca_certificate,
        headers: connection.headers,
        body: connection.body,
        message_template: connection.message_template,
        user_role: connection.user_role,
        model_role: connection.model_role,
        system_role: connection.system_role,
        system_prompt: connection.system_prompt,
        answer: connection.answer.name().to_owned(),
        text_path: connection.text_path,
        stop_path: connection.stop_path,
        cut_reasons: connection.cut_reasons,
        refused_reasons: connection.refused_reasons,
        error_path: connection.error_path,
        model_path: connection.model_path,
        input_tokens_path: connection.input_tokens_path,
        output_tokens_path: connection.output_tokens_path,
        done_marker: connection.done_marker,
    }
}

#[tokio::test]
async fn a_connection_is_saved_without_asking_the_service() {
    let test = logged_in().await;

    // Nothing listens on this port: saving does not need the service.
    let connection = Connection {
        name: "Ollama".to_owned(),
        model: "llama".to_owned(),
        url: "http://127.0.0.1:9/v1/chat/completions".to_owned(),
        ..Connection::template()
    };

    let saved = save_connection(&test.state, None, form(connection)).await.unwrap();

    assert_eq!(saved.notice, "The connection was saved.");
    assert_eq!(saved.config.connections.len(), 1);
    assert_eq!(saved.config.active, Some(saved.id));
    assert_eq!(saved.config.connections[0].name, "Ollama");
}

#[tokio::test]
async fn a_broken_field_is_named_for_the_page() {
    let test = logged_in().await;
    let connection = Connection {
        name: "Lab".to_owned(),
        model: "m".to_owned(),
        url: "http://10.0.0.5/v1".to_owned(),
        ..Connection::template()
    };

    let error = save_connection(&test.state, None, form(connection)).await.unwrap_err();

    assert!(matches!(&error, UiError::Connection(error) if error.field.name() == "url"));
    assert_eq!(serde_json::to_value(&error).unwrap()["field"], "url");
}

#[tokio::test]
async fn the_key_is_stored_and_never_given_back() {
    let test = logged_in().await;
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["OK"]))).await;
    let id = save_connection(&test.state, None, form(completions(&service))).await.unwrap().id;

    let view = store_key(&test.state, id, format!("  {API_KEY}  ")).await.unwrap();

    assert!(view.connections[0].has_key);
    assert!(!view.connections[0].key_unused);
    assert!(!serde_json::to_string(&view).unwrap().contains(API_KEY));

    let view = delete_key(&test.state, id).await.unwrap();
    assert!(!view.connections[0].has_key);
}

#[tokio::test]
async fn a_key_no_header_uses_is_reported() {
    let test = logged_in().await;
    let connection = Connection {
        name: "Local".to_owned(),
        model: "m".to_owned(),
        ..Connection::template()
    };
    let id = save_connection(&test.state, None, form(connection)).await.unwrap().id;

    let view = store_key(&test.state, id, API_KEY.to_owned()).await.unwrap();

    assert!(view.connections[0].key_unused);
}

#[tokio::test]
async fn a_check_reports_with_the_key_masked() {
    let test = logged_in().await;
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["O", "K"]))).await;
    let id = save_connection(&test.state, None, form(completions(&service))).await.unwrap().id;
    store_key(&test.state, id, API_KEY.to_owned()).await.unwrap();

    let report = check_connection(&test.state, id).await.unwrap();

    assert!(report.works, "{}", report.outcome);
    assert_eq!(report.text, "OK");
    assert_eq!(report.status, Some(200));
    assert!(!serde_json::to_string(&report).unwrap().contains(API_KEY));
    assert_eq!(service.received()[0].header("authorization"), Some(format!("Bearer {API_KEY}").as_str()));
}

#[tokio::test]
async fn a_check_without_the_needed_key_explains_it() {
    let test = logged_in().await;
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["OK"]))).await;
    let id = save_connection(&test.state, None, form(completions(&service))).await.unwrap().id;

    let report = check_connection(&test.state, id).await.unwrap();

    assert!(!report.works);
    assert!(report.outcome.contains("API key"), "{}", report.outcome);
    assert!(service.received().is_empty());
}

#[tokio::test]
async fn the_active_connection_is_chosen_and_deleted() {
    let test = logged_in().await;
    let template = |name: &str| Connection {
        name: name.to_owned(),
        model: "m".to_owned(),
        ..Connection::template()
    };

    let first = save_connection(&test.state, None, form(template("A"))).await.unwrap().id;
    let second = save_connection(&test.state, None, form(template("B"))).await.unwrap().id;

    assert_eq!(activate_connection(&test.state, second).await.unwrap().active, Some(second));
    assert!(matches!(activate_connection(&test.state, 99).await, Err(UiError::Invalid(_))));

    let view = delete_connection(&test.state, second).await.unwrap();
    assert_eq!(view.active, None);
    assert_eq!(view.connections.len(), 1);
    assert_eq!(view.connections[0].id, Some(first));
}

#[tokio::test]
async fn an_unknown_answer_mode_is_refused() {
    let test = logged_in().await;
    let mut sent = form(Connection {
        name: "X".to_owned(),
        model: "m".to_owned(),
        ..Connection::template()
    });
    sent.answer = "other".to_owned();

    assert!(matches!(save_connection(&test.state, None, sent).await, Err(UiError::Invalid(_))));
}

#[tokio::test]
async fn nothing_works_without_a_login() {
    let test = installed();

    assert!(matches!(ai_config(&test.state).await, Err(UiError::NotLoggedIn)));
    assert!(matches!(store_key(&test.state, 1, API_KEY.into()).await, Err(UiError::NotLoggedIn)));
}

#[tokio::test]
async fn an_instruction_is_given_for_a_field_of_the_form() {
    let test = logged_in().await;
    let template = || Connection {
        name: "X".to_owned(),
        ..Connection::template()
    };

    let text = field_instruction(&test.state, "text_path", form(template())).unwrap();
    assert!(text.contains("The field: Path to the text"));

    assert!(matches!(field_instruction(&test.state, "api_key", form(template())), Err(UiError::Invalid(_))));

    let nobody = installed();
    assert!(matches!(field_instruction(&nobody.state, "url", form(template())), Err(UiError::NotLoggedIn)));
}
