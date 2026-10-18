//! Unit tests of `client.rs`, against the fake service of
//! `test_support.rs`: real TLS (or plain HTTP) on 127.0.0.1, no
//! network.

use super::*;

use std::time::Duration;

use serde_json::json;

use crate::ai::api::test_support::{
    FakeAnswer, FakeService, checked, completions, completions_events, data, messages_events, messages_format, sse,
};
use crate::model::ai::Connection;

const KEY: &str = "sk-test-secret-key";

fn key() -> ApiKey {
    ApiKey::new(KEY.to_owned()).unwrap()
}

fn evaluator(text: &str) -> Turn {
    Turn { speaker: Speaker::Evaluator, text: text.to_owned() }
}

/// Asks for a reply and collects the pieces sent meanwhile.
async fn reply_of(connection: &Checked, turns: &[Turn]) -> (Reply, Vec<String>) {
    let (sender, mut receiver) = mpsc::channel(16);

    let collect = tokio::spawn(async move {
        let mut pieces = Vec::new();

        while let Some(piece) = receiver.recv().await {
            pieces.push(piece);
        }

        pieces
    });

    let reply = reply(connection, Some(&key()), turns, &sender).await;
    drop(sender);

    (reply, collect.await.unwrap())
}

#[tokio::test]
async fn a_completions_reply_arrives_in_pieces() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["Hel", "lo"]))).await;

    let (reply, pieces) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    assert_eq!(pieces, ["Hel", "lo"]);
    assert_eq!(reply.text, "Hello");
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Complete));
    assert_eq!(reply.model.as_deref(), Some("local-model"));
    assert_eq!((reply.input_tokens, reply.output_tokens), (10, 5));

    let request = &service.received()[0];
    assert!(request.head.starts_with("POST /v1/chat/completions "));
    assert_eq!(request.header("authorization"), Some(format!("Bearer {KEY}").as_str()));
    assert_eq!(request.header("content-type"), Some("application/json"));
    assert_eq!(request.body["model"], "local-model");
    assert_eq!(request.body["stream"], true);
    assert_eq!(request.body["messages"][1], json!({"role": "user", "content": "Hi"}));
    assert!(!request.body.to_string().contains(KEY));
}

#[tokio::test]
async fn a_reply_in_another_format_is_read_with_its_paths() {
    let service = FakeService::start(FakeAnswer::Stream(messages_events(&["Hi ", "there"]))).await;

    let (reply, _) = reply_of(&checked(messages_format(&service)), &[evaluator("Hello")]).await;

    assert_eq!(reply.text, "Hi there");
    assert_eq!(reply.model.as_deref(), Some("named-model"));
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Complete));

    let request = &service.received()[0];
    assert_eq!(request.header("x-api-key"), Some(KEY));
    assert_eq!(request.header("x-version"), Some("1"));
    assert_eq!(request.body["system"], crate::model::ai::DEFAULT_SYSTEM_PROMPT);
    assert_eq!(request.body["messages"], json!([{"role": "user", "content": "Hello"}]));
}

#[tokio::test]
async fn a_whole_answer_is_one_piece() {
    let body = json!({"choices": [{"message": {"content": "All at once"}, "finish_reason": "stop"}]});
    let service = FakeService::start(FakeAnswer::Json(200, body.to_string())).await;

    let connection = Connection {
        answer: AnswerMode::Whole,
        text_path: "choices[0].message.content".to_owned(),
        ..completions(&service)
    };

    let (reply, pieces) = reply_of(&checked(connection), &[evaluator("Hi")]).await;

    assert_eq!(pieces, ["All at once"]);
    assert_eq!(reply.end, ReplyEnd::Finished(StopReason::Complete));
}

#[tokio::test]
async fn plain_http_on_the_loopback_works() {
    let service = FakeService::start_plain(FakeAnswer::Stream(completions_events(&["local"]))).await;
    assert!(service.url.starts_with("http://127.0.0.1:"));

    let (reply, _) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    assert_eq!(reply.text, "local");
}

#[tokio::test]
async fn failures_are_told_apart_and_explained_by_the_error_path() {
    let body = json!({"error": {"message": "invalid key", "type": "auth"}}).to_string();
    let cases = [
        (401, AiError::KeyRejected("invalid key".to_owned())),
        (404, AiError::NotFound("invalid key".to_owned())),
        (429, AiError::LimitReached("invalid key".to_owned())),
        (503, AiError::ServiceFailed(503, "invalid key".to_owned())),
    ];

    for (status, expected) in cases {
        let service = FakeService::start(FakeAnswer::Json(status, body.clone())).await;
        let (reply, pieces) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

        assert!(pieces.is_empty());
        assert_eq!(reply.end, ReplyEnd::Failed(expected));
    }
}

#[tokio::test]
async fn the_key_is_masked_in_what_the_service_says() {
    let body = json!({"error": {"message": format!("the header was Bearer {KEY}")}}).to_string();
    let service = FakeService::start(FakeAnswer::Json(400, body)).await;

    let (reply, _) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    let ReplyEnd::Failed(error) = reply.end else {
        panic!("a 400 is a failure");
    };
    assert!(!error.to_string().contains(KEY));
    assert!(error.to_string().contains(MASK));
}

#[tokio::test]
async fn a_missing_key_fails_before_anything_is_sent() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["x"]))).await;
    let (sender, _receiver) = mpsc::channel(16);

    let reply = reply(&checked(completions(&service)), None, &[evaluator("Hi")], &sender).await;

    assert_eq!(reply.end, ReplyEnd::Failed(AiError::NoKey));
    assert!(service.received().is_empty());
}

#[tokio::test]
async fn a_redirect_is_not_followed() {
    let service = FakeService::start(FakeAnswer::Json(307, String::new())).await;

    let (reply, _) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    assert!(matches!(reply.end, ReplyEnd::Failed(AiError::UnexpectedAnswer(_))));
    assert_eq!(service.received().len(), 1);
}

#[tokio::test]
async fn tls_older_than_1_3_is_refused() {
    let service = FakeService::start_tls12(FakeAnswer::Json(200, "{}".to_owned())).await;

    let (reply, _) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    assert!(matches!(reply.end, ReplyEnd::Failed(AiError::NoConnection(_))));
    assert!(service.received().is_empty());
}

#[tokio::test]
async fn only_the_ca_certificate_of_the_connection_is_trusted() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["x"]))).await;
    let other = FakeService::start(FakeAnswer::Json(200, "{}".to_owned())).await;

    // The authority of another service.
    let foreign = Connection { ca_certificate: other.ca_pem.clone(), ..completions(&service) };
    let (reply, _) = reply_of(&checked(foreign), &[evaluator("Hi")]).await;
    assert!(matches!(reply.end, ReplyEnd::Failed(AiError::NoConnection(_))));

    // Without a certificate of its own, the system's are used, and
    // they do not know the test authority.
    let system = Connection { ca_certificate: String::new(), ..completions(&service) };
    let (reply, _) = reply_of(&checked(system), &[evaluator("Hi")]).await;
    assert!(matches!(reply.end, ReplyEnd::Failed(AiError::NoConnection(_))));

    assert!(service.received().is_empty());
}

#[tokio::test]
async fn an_error_in_the_middle_keeps_the_text_written_so_far() {
    let events = vec![
        data(&json!({"choices": [{"delta": {"content": "Part"}}]}).to_string()),
        data(&json!({"error": {"message": "overloaded"}}).to_string()),
    ];
    let service = FakeService::start(FakeAnswer::Stream(events)).await;

    let (reply, _) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    assert_eq!(reply.text, "Part");
    assert_eq!(reply.end, ReplyEnd::Failed(AiError::Reported("overloaded".to_owned())));
}

#[tokio::test]
async fn a_stream_that_breaks_off_is_a_failure() {
    let mut events = messages_events(&["Part"]);
    events.truncate(2);

    let service = FakeService::start(FakeAnswer::Stream(events)).await;
    let (reply, _) = reply_of(&checked(messages_format(&service)), &[evaluator("Hi")]).await;

    assert_eq!(reply.text, "Part");
    assert!(matches!(reply.end, ReplyEnd::Failed(AiError::NoConnection(_))));
}

#[tokio::test]
async fn dropping_the_receiver_stops_the_reply() {
    let events = vec![sse("content_block_delta", &json!({"delta": {"text": "First"}}))];

    let service = FakeService::start(FakeAnswer::StreamThenWait(events)).await;
    let connection = checked(messages_format(&service));
    let (sender, mut receiver) = mpsc::channel(16);

    let reply = tokio::spawn(async move { reply(&connection, Some(&key()), &[evaluator("Hi")], &sender).await });

    // The first piece arrives; then the reader goes away, as when
    // the evaluator presses "Stop".
    assert_eq!(receiver.recv().await.as_deref(), Some("First"));
    drop(receiver);

    let reply = tokio::time::timeout(Duration::from_secs(10), reply).await.unwrap().unwrap();

    assert_eq!(reply.end, ReplyEnd::Cancelled);
    assert_eq!(reply.text, "First");
}

#[tokio::test]
async fn a_check_reports_the_request_and_the_answer_with_the_key_masked() {
    let service = FakeService::start(FakeAnswer::Stream(completions_events(&["OK"]))).await;

    let report = check(&checked(completions(&service)), Some(&key())).await;

    assert_eq!(report.status, Some(200));
    assert_eq!(report.reply.text, "OK");
    assert_eq!(report.reply.end, ReplyEnd::Finished(StopReason::Complete));
    assert!(report.raw.contains("[DONE]"));

    let (name, value) = &report.request.headers[0];
    assert_eq!((name.as_str(), value.as_str()), ("authorization", format!("Bearer {MASK}").as_str()));
    assert_eq!(report.request.body["messages"][1]["content"], CHECK_MESSAGE);

    // The service received the real key.
    assert_eq!(service.received()[0].header("authorization"), Some(format!("Bearer {KEY}").as_str()));
}

#[tokio::test]
async fn a_failed_check_shows_what_the_service_said() {
    let service = FakeService::start(FakeAnswer::Json(400, format!("echo of {KEY}"))).await;

    let report = check(&checked(completions(&service)), Some(&key())).await;

    assert_eq!(report.status, Some(400));
    assert_eq!(report.raw, format!("echo of {MASK}"));
    assert!(matches!(report.reply.end, ReplyEnd::Failed(AiError::Rejected(400, _))));
}

#[tokio::test]
async fn without_a_model_named_by_the_service_the_connection_names_it() {
    let events = vec![data(&json!({"choices": [{"delta": {"content": "x"}}]}).to_string()), data("[DONE]")];
    let service = FakeService::start(FakeAnswer::Stream(events)).await;

    let (reply, _) = reply_of(&checked(completions(&service)), &[evaluator("Hi")]).await;

    assert_eq!(reply.model.as_deref(), Some("local-model"));
}
