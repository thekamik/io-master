//! Unit tests of `anthropic.rs`, against the fake service of
//! `test_support.rs`: real TLS on 127.0.0.1, no network.

use super::*;

use std::time::Duration;

use serde_json::json;

use crate::ai::test_support::{FakeAnswer, FakeAnthropic, reply_events, sse};

fn key() -> ApiKey {
    ApiKey::new("sk-ant-test-key".to_owned()).unwrap()
}

fn evaluator(text: &str) -> Turn {
    Turn {
        speaker: Speaker::Evaluator,
        text: text.to_owned(),
    }
}

fn model(text: &str) -> Turn {
    Turn {
        speaker: Speaker::Model,
        text: text.to_owned(),
    }
}

/// The JSON body of an error answer.
fn error_body(kind: &str, message: &str) -> String {
    json!({"type": "error", "error": {"type": kind, "message": message}}).to_string()
}

/// Asks for a reply and collects the pieces sent meanwhile.
async fn reply_of(client: &Anthropic, model: Model, turns: &[Turn]) -> (Reply, Vec<String>) {
    let (sender, mut receiver) = mpsc::channel(16);

    let collect = tokio::spawn(async move {
        let mut pieces = Vec::new();

        while let Some(piece) = receiver.recv().await {
            pieces.push(piece);
        }

        pieces
    });

    let reply = client.reply(&key(), model, turns, &sender).await;
    drop(sender);

    (reply, collect.await.unwrap())
}

#[tokio::test]
async fn a_good_key_passes_the_check_and_is_sent_only_in_its_header() {
    let service = FakeAnthropic::start(FakeAnswer::Json(200, json!({"type": "message"}).to_string())).await;

    service.client().check(&key(), Model::Haiku).await.unwrap();

    let requests = service.received();
    assert_eq!(requests.len(), 1);

    let request = &requests[0];
    assert!(request.head.starts_with("POST /v1/messages "));
    assert_eq!(request.header("x-api-key"), Some("sk-ant-test-key"));
    assert_eq!(request.header("anthropic-version"), Some("2023-06-01"));
    assert_eq!(request.body["model"], "claude-haiku-5-5");
    assert_eq!(request.body["max_tokens"], CHECK_MAX_TOKENS);
    assert!(!request.body.to_string().contains("sk-ant-test-key"));

    // Haiku has no fallback.
    assert_eq!(request.header("anthropic-beta"), None);
    assert!(request.body.get("fallbacks").is_none());
}

#[tokio::test]
async fn opus_asks_for_the_fallback() {
    let service = FakeAnthropic::start(FakeAnswer::Json(200, "{}".to_owned())).await;

    service.client().check(&key(), Model::Opus).await.unwrap();

    let request = &service.received()[0];
    assert_eq!(request.header("anthropic-beta"), Some(FALLBACK_BETA));
    assert_eq!(request.body["fallbacks"], "default");
}

#[tokio::test]
async fn failed_checks_are_told_apart() {
    let cases = [
        (401, error_body("authentication_error", "invalid x-api-key"), AiError::KeyRejected),
        (404, error_body("not_found_error", "model: claude-x"), AiError::UnknownModel),
        (429, error_body("rate_limit_error", "slow down"), AiError::LimitReached),
        (529, error_body("overloaded_error", "Overloaded"), AiError::Overloaded),
    ];

    for (status, body, expected) in cases {
        let service = FakeAnthropic::start(FakeAnswer::Json(status, body)).await;

        assert_eq!(service.client().check(&key(), Model::Sonnet).await.unwrap_err(), expected);
    }
}

#[tokio::test]
async fn a_redirect_is_not_followed() {
    let service = FakeAnthropic::start(FakeAnswer::Json(307, String::new())).await;

    assert!(matches!(
        service.client().check(&key(), Model::Haiku).await,
        Err(AiError::UnexpectedAnswer(_))
    ));
    assert_eq!(service.received().len(), 1);
}

#[tokio::test]
async fn tls_older_than_1_3_is_refused() {
    let service = FakeAnthropic::start_tls12(FakeAnswer::Json(200, "{}".to_owned())).await;

    assert!(matches!(
        service.client().check(&key(), Model::Haiku).await,
        Err(AiError::NoConnection(_))
    ));
    assert!(service.received().is_empty());
}

#[tokio::test]
async fn an_untrusted_certificate_is_refused() {
    let service = FakeAnthropic::start(FakeAnswer::Json(200, "{}".to_owned())).await;
    let other = FakeAnthropic::start(FakeAnswer::Json(200, "{}".to_owned())).await;

    // A client that trusts the authority of another service.
    let client = Anthropic::with_service(&service.url, other.root());

    assert!(matches!(client.check(&key(), Model::Haiku).await, Err(AiError::NoConnection(_))));
    assert!(service.received().is_empty());
}

#[tokio::test]
async fn a_reply_arrives_in_pieces() {
    let service = FakeAnthropic::start(FakeAnswer::Stream(reply_events(&["Hel", "lo"]))).await;

    let (reply, pieces) = reply_of(&service.client(), Model::Opus, &[evaluator("Hi")]).await;

    assert_eq!(pieces, ["Hel", "lo"]);
    assert_eq!(reply.text, "Hello");
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::EndTurn));
    assert_eq!(reply.model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!((reply.input_tokens, reply.output_tokens), (10, 5));

    let request = &service.received()[0];
    assert_eq!(request.body["stream"], true);
    assert_eq!(request.body["max_tokens"], MAX_TOKENS);
    assert_eq!(request.body["messages"], json!([{"role": "user", "content": "Hi"}]));
}

#[tokio::test]
async fn an_error_in_the_middle_keeps_the_text_written_so_far() {
    let mut events = reply_events(&["Part"]);
    events.truncate(3);
    events.push(sse("error", &json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}})));

    let service = FakeAnthropic::start(FakeAnswer::Stream(events)).await;
    let (reply, _) = reply_of(&service.client(), Model::Opus, &[evaluator("Hi")]).await;

    assert_eq!(reply.text, "Part");
    assert_eq!(reply.end, ReplyEnd::Failed(AiError::Overloaded));
}

#[tokio::test]
async fn a_stream_that_breaks_off_is_a_failure() {
    let mut events = reply_events(&["Part"]);
    events.truncate(3);

    let service = FakeAnthropic::start(FakeAnswer::Stream(events)).await;
    let (reply, _) = reply_of(&service.client(), Model::Opus, &[evaluator("Hi")]).await;

    assert_eq!(reply.text, "Part");
    assert!(matches!(reply.end, ReplyEnd::Failed(AiError::NoConnection(_))));
}

#[tokio::test]
async fn a_rejected_request_is_a_failure_without_text() {
    let service = FakeAnthropic::start(FakeAnswer::Json(401, error_body("authentication_error", "invalid"))).await;
    let (reply, pieces) = reply_of(&service.client(), Model::Opus, &[evaluator("Hi")]).await;

    assert!(pieces.is_empty());
    assert_eq!(reply.end, ReplyEnd::Failed(AiError::KeyRejected));
}

#[tokio::test]
async fn dropping_the_receiver_stops_the_reply() {
    let mut events = reply_events(&["First"]);
    events.truncate(3);

    let service = FakeAnthropic::start(FakeAnswer::StreamThenWait(events)).await;
    let client = service.client();
    let (sender, mut receiver) = mpsc::channel(16);

    let reply = tokio::spawn(async move { client.reply(&key(), Model::Opus, &[evaluator("Hi")], &sender).await });

    // The first piece arrives; then the reader goes away, as when
    // the evaluator presses "Stop".
    assert_eq!(receiver.recv().await.as_deref(), Some("First"));
    drop(receiver);

    let reply = tokio::time::timeout(Duration::from_secs(10), reply).await.unwrap().unwrap();

    assert_eq!(reply.end, ReplyEnd::Cancelled);
    assert_eq!(reply.text, "First");
}

#[test]
fn turns_become_alternating_messages() {
    let turns = [
        model("a greeting the model never got"),
        evaluator("first"),
        evaluator("second, after a failed reply"),
        model("   "),
        model("answer"),
        evaluator("next"),
    ];

    assert_eq!(
        messages(&turns),
        [
            json!({"role": "user", "content": "first\n\nsecond, after a failed reply"}),
            json!({"role": "assistant", "content": "answer"}),
            json!({"role": "user", "content": "next"}),
        ]
    );
}
