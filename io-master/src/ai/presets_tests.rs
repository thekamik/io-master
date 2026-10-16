//! Unit tests of `presets.rs`: every preset keeps the rules, and
//! reads a stream in the form of its service, sent by the fake
//! service of `test_support.rs` (the address of the preset replaced
//! by that of the fake service, everything else as in the preset).

use super::*;

use serde_json::json;
use tokio::sync::mpsc;

use crate::ai::api_key::ApiKey;
use crate::ai::client::{self, Reply, ReplyEnd};
use crate::ai::request::{Speaker, Turn};
use crate::ai::response::StopReason;
use crate::ai::test_support::{FakeAnswer, FakeRequest, FakeService, completions_events, data, messages_events};

fn preset(id: &str) -> Preset {
    all().into_iter().find(|preset| preset.id == id).unwrap()
}

/// The preset sent to `service`: its scheme, host and port replaced,
/// its path kept; the CA certificate of the fake service trusted.
fn at(service: &FakeService, mut connection: Connection) -> Connection {
    let path = connection.url.splitn(4, '/').nth(3).unwrap_or_default().to_owned();

    connection.url = format!("{}/{path}", service.url);
    connection.ca_certificate = service.ca_pem.clone();
    connection
}

/// Asks for a reply over the preset at the fake service.
async fn ask(answer: FakeAnswer, id: &str) -> (Reply, FakeRequest) {
    let service = FakeService::start(answer).await;
    let connection = at(&service, preset(id).connection).check().unwrap();
    let key = ApiKey::new("sk-test".to_owned()).unwrap();
    let turns = [Turn {
        speaker: Speaker::Evaluator,
        text: "Hi".to_owned(),
    }];

    let (pieces, mut receiver) = mpsc::channel(64);
    let draining = tokio::spawn(async move { while receiver.recv().await.is_some() {} });
    let reply = client::reply(&connection, Some(&key), &turns, &pieces).await;
    drop(pieces);
    draining.await.unwrap();

    (reply, service.received().remove(0))
}

#[test]
fn every_preset_keeps_the_rules_and_has_its_own_id() {
    let presets = all();

    for preset in &presets {
        preset.connection.check().unwrap_or_else(|error| panic!("{}: {error}", preset.id));
    }

    let mut ids: Vec<&str> = presets.iter().map(|preset| preset.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), presets.len());
}

#[test]
fn the_local_preset_stays_on_this_computer_and_needs_no_key() {
    let connection = preset("local-gemma").connection.check().unwrap();

    assert!(connection.url.starts_with("http://127.0.0.1:"));
    assert!(!connection.needs_key());

    for id in ["anthropic", "anthropic-haiku", "openai", "google"] {
        let connection = preset(id).connection.check().unwrap();

        assert!(connection.url.starts_with("https://"), "{id}");
        assert!(connection.needs_key(), "{id}");
    }
}

#[tokio::test]
async fn the_anthropic_preset_reads_its_stream() {
    let (reply, request) = ask(FakeAnswer::Stream(messages_events(&["Hel", "lo"])), "anthropic").await;

    assert_eq!(reply.text, "Hello");
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Complete));
    assert_eq!(request.header("x-api-key"), Some("sk-test"));
    assert_eq!(request.body["messages"], json!([{"role": "user", "content": "Hi"}]));
    assert_eq!(request.body["fallbacks"], "default");
}

#[tokio::test]
async fn the_haiku_preset_reads_its_stream_without_the_fallback() {
    let (reply, request) = ask(FakeAnswer::Stream(messages_events(&["Hel", "lo"])), "anthropic-haiku").await;

    assert_eq!(reply.text, "Hello");
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Complete));
    assert_eq!(request.header("x-api-key"), Some("sk-test"));
    assert_eq!(request.header("anthropic-beta"), None);
    assert_eq!(request.body["model"], "claude-haiku-5-5");
    assert_eq!(request.body["messages"], json!([{"role": "user", "content": "Hi"}]));
    assert!(request.body.get("fallbacks").is_none());
}

/// The shared fields of the Messages API give the Opus preset the
/// same headers and body it had when they were written out in full.
#[test]
fn the_opus_preset_keeps_its_headers_and_body() {
    let connection = preset("anthropic").connection;

    assert_eq!(
        connection.headers,
        "content-type: application/json\nx-api-key: {{api_key}}\nanthropic-version: 2023-06-01\n\
         anthropic-beta: server-side-fallback-2026-07-01"
    );
    assert_eq!(
        connection.body,
        "{\n  \"model\": \"{{model}}\",\n  \"max_tokens\": 64000,\n  \"system\": \"{{system}}\",\n  \
         \"messages\": \"{{messages}}\",\n  \"stream\": true,\n  \"fallbacks\": \"default\"\n}"
    );
}

#[tokio::test]
async fn the_openai_preset_reads_its_stream() {
    let (reply, request) = ask(FakeAnswer::Stream(completions_events(&["Hel", "lo"])), "openai").await;

    assert_eq!(reply.text, "Hello");
    assert_eq!((reply.input_tokens, reply.output_tokens), (10, 5));
    assert_eq!(request.header("authorization"), Some("Bearer sk-test"));
    assert_eq!(request.body["messages"][0]["role"], "system");
    assert_eq!(request.body["stream_options"]["include_usage"], true);
}

#[tokio::test]
async fn the_google_preset_reads_its_stream() {
    let chunk = |text: &str| {
        data(&json!({"candidates": [{"content": {"role": "model", "parts": [{"text": text}]}}], "modelVersion": "gemini-4-argon"}).to_string())
    };
    let last = data(
        &json!({
            "candidates": [{"content": {"role": "model", "parts": [{"text": "!"}]}, "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 9, "candidatesTokenCount": 3},
            "modelVersion": "gemini-4-argon"
        })
        .to_string(),
    );

    let (reply, request) = ask(FakeAnswer::Stream(vec![chunk("Hel"), chunk("lo"), last]), "google").await;

    assert_eq!(reply.text, "Hello!");
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Complete));
    assert_eq!(reply.model.as_deref(), Some("gemini-4-argon"));
    assert_eq!((reply.input_tokens, reply.output_tokens), (9, 3));

    assert!(request.head.starts_with("POST /v1beta/models/gemini-4-argon:streamGenerateContent?alt=sse "));
    assert_eq!(request.header("x-goog-api-key"), Some("sk-test"));
    assert_eq!(request.body["contents"], json!([{"role": "user", "parts": [{"text": "Hi"}]}]));
    assert_eq!(
        request.body["systemInstruction"]["parts"][0]["text"],
        crate::ai::connection::DEFAULT_SYSTEM_PROMPT
    );
}

#[tokio::test]
async fn a_declined_gemini_reply_is_refused() {
    let blocked = data(&json!({"candidates": [{"finishReason": "SAFETY"}]}).to_string());

    let (reply, _) = ask(FakeAnswer::Stream(vec![blocked]), "google").await;

    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Refused));
}
