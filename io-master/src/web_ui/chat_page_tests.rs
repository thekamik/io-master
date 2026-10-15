//! Unit tests of `chat_page.rs` and of the AI section of
//! `settings.rs`: the HTML, and the routes as a browser uses them,
//! with a fake Anthropic service (`ai/test_support.rs`).

use super::*;

use axum::http::Method;
use axum::http::header::{CONTENT_TYPE, LOCATION};
use serde_json::json;

use crate::ai::test_support::{FakeAnswer, FakeAnthropic, reply_events};
use crate::installation::installation::Role;
use crate::web_ui::test_support::{self as web, TestServer, installed, log_in, request, text};

/// A key as an evaluator pastes it.
const API_KEY: &str = "sk-ant-api03-secret-of-anna";

/// A server with the users anna and jan, and the session of anna.
async fn server_with_anna() -> (TestServer, String) {
    let server = installed(Role::Workstation, &["anna", "jan"]);
    let token = log_in(&server.state, "anna").await;

    (server, token)
}

/// A fake service that accepts every check.
async fn accepting() -> FakeAnthropic {
    FakeAnthropic::start(FakeAnswer::Json(200, json!({"type": "message"}).to_string())).await
}

/// Stores the API key of the session through the Settings form,
/// checked by a fake service that accepts it.
async fn store_key(server: &mut TestServer, token: &str) {
    let service = accepting().await;
    server.state.ai = service.client();

    let body = format!("api_key={API_KEY}");
    let response = web::send(&server.state, request(Method::POST, "/settings/ai/key", Some(token), &body)).await;

    assert_eq!(response.status(), StatusCode::OK);
}

/// Sends a message of the chat and returns the status and the body
/// (the whole stream of events).
async fn send_message(server: &TestServer, token: &str, chat: &str, message: &str) -> (StatusCode, String) {
    let body = format!("chat={chat}&text={}", message.replace(' ', "+"));
    let response = web::send(&server.state, request(Method::POST, "/chat/send", Some(token), &body)).await;
    let status = response.status();

    (status, text(response).await)
}

/* ---------------------------------------------------------------
 * The API key in Settings
 * ------------------------------------------------------------- */

#[tokio::test]
async fn a_checked_key_is_stored_and_never_shown() {
    let (mut server, token) = server_with_anna().await;
    let service = accepting().await;
    server.state.ai = service.client();

    let body = format!("api_key=++{API_KEY}++");
    let response = web::send(&server.state, request(Method::POST, "/settings/ai/key", Some(&token), &body)).await;
    let html = text(response).await;

    assert!(html.contains(
        "The model Claude Opus 5.5 is configured correctly and can be used in the chat or to support the evaluation."
    ));
    assert!(html.contains("An API key of Anthropic is stored."));
    assert!(!html.contains("secret-of-anna"));
    assert!(!html.contains("sk-ant"));

    // The key went to the service, without the spaces around it.
    assert_eq!(service.received()[0].header("x-api-key"), Some(API_KEY));

    // Later visits of Settings do not show it either.
    let html = text(web::send(&server.state, request(Method::GET, "/settings", Some(&token), "")).await).await;
    assert!(html.contains("Delete the API key"));
    assert!(!html.contains("secret-of-anna"));
}

#[tokio::test]
async fn a_rejected_key_is_not_stored() {
    let (mut server, token) = server_with_anna().await;
    let error = json!({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}});
    let service = FakeAnthropic::start(FakeAnswer::Json(401, error.to_string())).await;
    server.state.ai = service.client();

    let body = format!("api_key={API_KEY}");
    let response = web::send(&server.state, request(Method::POST, "/settings/ai/key", Some(&token), &body)).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let html = text(response).await;
    assert!(html.contains("The key was not stored: Anthropic does not accept the API key."));
    assert!(html.contains("No API key is stored"));
    assert!(!html.contains("secret-of-anna"));
}

#[tokio::test]
async fn an_unreachable_service_stores_nothing() {
    // The offline service of the test server: nothing listens.
    let (server, token) = server_with_anna().await;

    let body = format!("api_key={API_KEY}");
    let response = web::send(&server.state, request(Method::POST, "/settings/ai/key", Some(&token), &body)).await;

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(text(response).await.contains("Anthropic cannot be reached"));
}

#[tokio::test]
async fn the_key_is_deleted() {
    let (mut server, token) = server_with_anna().await;
    store_key(&mut server, &token).await;

    let response = web::send(&server.state, request(Method::POST, "/settings/ai/key/delete", Some(&token), "")).await;
    let html = text(response).await;

    assert!(html.contains("The API key was deleted."));
    assert!(html.contains("No API key is stored"));
}

#[tokio::test]
async fn the_key_is_kept_per_user() {
    let (mut server, anna) = server_with_anna().await;
    store_key(&mut server, &anna).await;

    let jan = log_in(&server.state, "jan").await;
    let html = text(web::send(&server.state, request(Method::GET, "/settings", Some(&jan), "")).await).await;

    assert!(html.contains("No API key is stored"));
}

/* ---------------------------------------------------------------
 * The chat
 * ------------------------------------------------------------- */

#[tokio::test]
async fn without_a_key_the_chat_says_where_to_enter_one() {
    let (server, token) = server_with_anna().await;

    let html = text(web::send(&server.state, request(Method::GET, "/chat", Some(&token), "")).await).await;
    assert!(html.contains("No API key of Anthropic is stored"));
    assert!(html.contains("<script src=\"/static/chat.js\" defer></script>"));

    let (status, body) = send_message(&server, &token, "", "Hello").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("No API key is stored"));
}

#[tokio::test]
async fn a_message_gets_a_streamed_reply_that_is_stored() {
    let (mut server, token) = server_with_anna().await;
    store_key(&mut server, &token).await;

    let service = FakeAnthropic::start(FakeAnswer::Stream(reply_events(&["ADV_FSP is ", "<the functional spec>"]))).await;
    server.state.ai = service.client();

    let (status, events) = send_message(&server, &token, "", "What is ADV_FSP?").await;

    assert_eq!(status, StatusCode::OK);
    assert!(events.contains("event: chat\ndata: {\"id\":1,"), "{events}");
    assert!(events.contains("event: text\ndata: {\"text\":\"ADV_FSP is \"}"));
    assert!(events.contains("event: end\ndata: {\"ending\":\"complete\",\"notice\":null}"));

    // The request carried the key and the message.
    let request_body = &service.received()[0].body;
    assert_eq!(request_body["messages"], json!([{"role": "user", "content": "What is ADV_FSP?"}]));

    // The chat is in the list, with the reply, escaped.
    let html = text(web::send(&server.state, request(Method::GET, "/chat/1", Some(&token), "")).await).await;
    assert!(html.contains("What is ADV_FSP?"));
    assert!(html.contains("ADV_FSP is &lt;the functional spec&gt;"));
    assert!(html.contains("claude-opus-5-5 · "));

    // A further message continues the chat with its history.
    let (_, _) = send_message(&server, &token, "1", "And ADV_TDS?").await;
    let messages = &service.received()[1].body["messages"];

    assert_eq!(messages.as_array().unwrap().len(), 3);
    assert_eq!(messages[1], json!({"role": "assistant", "content": "ADV_FSP is <the functional spec>"}));
}

#[tokio::test]
async fn a_failed_reply_is_stored_with_its_explanation() {
    let (mut server, token) = server_with_anna().await;
    store_key(&mut server, &token).await;

    let error = json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}});
    let service = FakeAnthropic::start(FakeAnswer::Json(529, error.to_string())).await;
    server.state.ai = service.client();

    let (_, events) = send_message(&server, &token, "", "Hello").await;
    assert!(events.contains("\"ending\":\"failed\""));
    assert!(events.contains("Anthropic is overloaded at the moment"));

    let html = text(web::send(&server.state, request(Method::GET, "/chat/1", Some(&token), "")).await).await;
    assert!(html.contains("The reply failed: Anthropic is overloaded at the moment; try again later."));
}

#[tokio::test]
async fn chats_are_private_and_can_be_deleted() {
    let (mut server, anna) = server_with_anna().await;
    store_key(&mut server, &anna).await;

    let service = FakeAnthropic::start(FakeAnswer::Stream(reply_events(&["Hi"]))).await;
    server.state.ai = service.client();
    send_message(&server, &anna, "", "Private question").await;

    // Jan has a database of his own: chat 1 is not there.
    let jan = log_in(&server.state, "jan").await;
    let response = web::send(&server.state, request(Method::GET, "/chat/1", Some(&jan), "")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let (status, _) = send_message(&server, &jan, "1", "Let me in").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "jan has no key");

    // Anna deletes her chat.
    let response = web::send(&server.state, request(Method::POST, "/chat/1/delete", Some(&anna), "")).await;
    assert_eq!(response.headers()[LOCATION], "/chat");

    let response = web::send(&server.state, request(Method::GET, "/chat/1", Some(&anna), "")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn bad_messages_are_refused() {
    let (mut server, token) = server_with_anna().await;
    store_key(&mut server, &token).await;

    assert_eq!(send_message(&server, &token, "", "   ").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(send_message(&server, &token, "x", "Hello").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(send_message(&server, &token, "99", "Hello").await.0, StatusCode::NOT_FOUND);
}

/* ---------------------------------------------------------------
 * The settings of the chat
 * ------------------------------------------------------------- */

#[tokio::test]
async fn a_model_is_checked_and_stored() {
    let (mut server, token) = server_with_anna().await;
    store_key(&mut server, &token).await;

    let service = accepting().await;
    server.state.ai = service.client();

    let response = web::send(
        &server.state,
        request(Method::POST, "/chat/settings", Some(&token), "model=claude-haiku-5-5"),
    )
    .await;

    assert_eq!(response.headers()[LOCATION], "/chat");
    assert_eq!(service.received()[0].body["model"], "claude-haiku-5-5");

    let html = text(web::send(&server.state, request(Method::GET, "/chat", Some(&token), "")).await).await;
    assert!(html.contains("Chat settings: Claude Haiku 5.5"));
    assert!(html.contains("value=\"claude-haiku-5-5\" checked"));
}

#[tokio::test]
async fn a_model_is_not_stored_without_a_key_or_when_refused() {
    let (mut server, token) = server_with_anna().await;

    let response = web::send(
        &server.state,
        request(Method::POST, "/chat/settings", Some(&token), "model=claude-haiku-5-5"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(text(response).await.contains("The model was not saved: no API key is stored"));

    store_key(&mut server, &token).await;

    let error = json!({"type": "error", "error": {"type": "not_found_error", "message": "model"}});
    let service = FakeAnthropic::start(FakeAnswer::Json(404, error.to_string())).await;
    server.state.ai = service.client();

    let response = web::send(
        &server.state,
        request(Method::POST, "/chat/settings", Some(&token), "model=claude-sonnet-5-5"),
    )
    .await;
    let html = text(response).await;

    assert!(html.contains("The model was not saved: the model is not available"));
    assert!(html.contains("Chat settings: Claude Opus 5.5"));

    let response = web::send(&server.state, request(Method::POST, "/chat/settings", Some(&token), "model=gpt")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_script_is_served() {
    let server = installed(Role::Workstation, &["anna"]);
    let response = web::send(&server.state, request(Method::GET, "/static/chat.js", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "text/javascript; charset=utf-8");
}

/* ---------------------------------------------------------------
 * The HTML
 * ------------------------------------------------------------- */

#[test]
fn messages_are_escaped_and_endings_explained() {
    let message = ChatMessage {
        speaker: Speaker::Model,
        text: "<script>alert(1)</script>".to_owned(),
        created: "2026-10-14 15:04".to_owned(),
        model: Some("claude-opus-5-5".to_owned()),
        ending: Ending::Failed,
        note: Some("Anthropic <failed>".to_owned()),
    };

    let html = message_html(&message);

    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("The reply failed: Anthropic &lt;failed&gt;."));
    assert!(!html.contains("<script>"));
}

#[test]
fn the_settings_name_the_fallback() {
    let html = settings_form(Model::Sonnet);

    assert!(html.contains("value=\"claude-sonnet-5-5\" checked"));
    assert!(html.contains("server-side fallback"));
}
