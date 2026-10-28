//! The connection to the AI service of a connection
//! (`ai/api/connection.rs`): a reply to a conversation while it is
//! written, and the check of a connection with a report for the
//! evaluator.
//!
//! **The connection protects the key and the conversation** (the
//! rules of `connection.rs` are checked before a request is made):
//!
//! - TLS 1.3 or newer for `https://`, the certificate verified
//!   against the trusted certificates of the system, or only against
//!   the CA certificate of the connection;
//! - redirects are never followed, so the key and the conversation
//!   go to the address of the connection and nowhere else;
//! - proxies of the environment (`HTTPS_PROXY`) are not used;
//! - the key is marked "sensitive" in its header, and masked in
//!   every text taken from an answer (errors, the report of a check).
//!
//! **A reply can be stopped by the caller**: [`reply`] sends the
//! pieces of the reply into a channel, and stops as soon as the
//! receiving end of the channel is dropped (the evaluator pressed
//! "Stop", or logged out). The text written up to then is returned,
//! so that it can be kept.

use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::tls::Version;
use reqwest::{Client, Response};
use tokio::sync::mpsc;

use super::ai_error::AiError;
use super::request::{self, MASK, Request};
use super::response::{AnswerReader, StopReason};
use super::sse::SseDecoder;
use crate::ai::api::connection::Checked;
use crate::model_old::ai::AnswerMode;
use crate::model_old::ai::{Speaker, Turn};
use crate::model_old::api_key::ApiKey;

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long a stream may stay silent. A local model on a slow
/// computer may think long before the first piece, so this is
/// generous; a longer silence means a broken connection.
const STREAM_SILENCE: Duration = Duration::from_secs(300);

/// How long a whole answer (one JSON) may take: the service sends
/// nothing until the reply is complete.
const WHOLE_SILENCE: Duration = Duration::from_secs(900);

/// How long the whole check of a connection may take.
const CHECK_TIMEOUT: Duration = Duration::from_secs(120);

/// The largest answer that is read, in bytes: the text of a reply,
/// a whole answer, or the body of an error. Far more than any reply;
/// it only stops a broken service from filling the memory.
const MAX_ANSWER: usize = 8 * 1024 * 1024;

/// How much of the raw answer the report of a check shows, in bytes.
const REPORT_LENGTH: usize = 16 * 1024;

/// The message of a check.
const CHECK_MESSAGE: &str = "Reply with the word OK.";

/// How a reply ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyEnd {
    /// The model stopped writing, for the given reason.
    Finished(StopReason),
    /// The caller stopped the reply.
    Cancelled,
    /// The request or the reply failed.
    Failed(AiError),
}

/// A reply of the model, complete or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// The text written; may be partial (see `end`).
    pub text: String,
    /// The model that answered, as the service named it, else the
    /// model of the connection; `None` when neither is known.
    pub model: Option<String>,
    /// Tokens of the request (the whole conversation), when the
    /// service reports them.
    pub input_tokens: u64,
    /// Tokens of the reply, when the service reports them.
    pub output_tokens: u64,
    pub end: ReplyEnd,
}

/// What a check found, to be shown to the evaluator.
#[derive(Debug)]
pub struct CheckReport {
    /// The request as it was sent, the key masked.
    pub request: Request,
    /// The HTTP status of the answer; `None` when none came.
    pub status: Option<u16>,
    /// The beginning of the answer as it came, the key masked.
    pub raw: String,
    /// What the paths of the connection found, and how it ended.
    pub reply: Reply,
}

/// Asks the model for the next reply of a conversation, and sends
/// the reply into `pieces` while it is written. Stops when the
/// receiver of `pieces` is dropped (see the top of this file). Never
/// fails as a whole: how the reply ended is in [`Reply::end`], next
/// to what was written until then.
pub async fn reply(connection: &Checked, key: Option<&ApiKey>, turns: &[Turn], pieces: &mpsc::Sender<String>) -> Reply {
    exchange(connection, key, turns, pieces, &mut None).await
}

/// Checks a connection with one short message, and reports what was
/// sent, what came back and what the paths found in it.
pub async fn check(connection: &Checked, key: Option<&ApiKey>) -> CheckReport {
    let turns = [Turn { speaker: Speaker::Evaluator, text: CHECK_MESSAGE.to_owned() }];

    let (pieces, mut receiver) = mpsc::channel::<String>(64);
    let mut raw = Some(Raw::default());

    // The pieces are only drained: the report shows the whole text.
    let asking = async {
        let reply = tokio::time::timeout(CHECK_TIMEOUT, exchange(connection, key, &turns, &pieces, &mut raw)).await;
        drop(pieces);
        reply
    };
    let draining = async { while receiver.recv().await.is_some() {} };

    let (reply, ()) = tokio::join!(asking, draining);

    let reply = reply.unwrap_or_else(|_| Reply {
        text: String::new(),
        model: None,
        input_tokens: 0,
        output_tokens: 0,
        end: ReplyEnd::Failed(AiError::NoConnection("no answer in time".to_owned())),
    });

    let raw = raw.unwrap_or_default();

    CheckReport {
        request: request::preview(connection, &turns),
        status: raw.status,
        raw: mask(&String::from_utf8_lossy(&raw.bytes), key),
        reply,
    }
}

/// The beginning of an answer as it came, kept for the report of a
/// check.
#[derive(Debug, Default)]
struct Raw {
    status: Option<u16>,
    bytes: Vec<u8>,
}

impl Raw {
    fn keep(raw: &mut Option<Raw>, bytes: &[u8]) {
        if let Some(raw) = raw {
            let room = REPORT_LENGTH.saturating_sub(raw.bytes.len());
            raw.bytes.extend_from_slice(&bytes[..bytes.len().min(room)]);
        }
    }
}

/// The work of [`reply`] and [`check`]; `raw`, when given, keeps the
/// beginning of the answer.
async fn exchange(
    connection: &Checked,
    key: Option<&ApiKey>,
    turns: &[Turn],
    pieces: &mpsc::Sender<String>,
    raw: &mut Option<Raw>,
) -> Reply {
    let mut reader = AnswerReader::new(connection);

    // Everything that can fail before the request is sent.
    let prepared = request::request(connection, turns, key).and_then(|request| Ok((http_client(connection)?, request)));

    let (http, request) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return finish(connection, reader, ReplyEnd::Failed(error)),
    };

    let sent = tokio::select! {
        sent = send(&http, request) => sent,
        () = pieces.closed() => return finish(connection, reader, ReplyEnd::Cancelled),
    };

    let mut response = match sent {
        Ok(response) => response,
        Err(error) => return finish(connection, reader, ReplyEnd::Failed(masked(error, key))),
    };

    let status = response.status().as_u16();

    if let Some(raw) = raw {
        raw.status = Some(status);
    }

    if !response.status().is_success() {
        let body = match read_all(&mut response, raw, pieces).await {
            Read::Bytes(body) => body,
            Read::Cancelled => return finish(connection, reader, ReplyEnd::Cancelled),
            Read::Failed(error) => return finish(connection, reader, ReplyEnd::Failed(masked(error, key))),
        };

        let error = AiError::from_answer(status, &body, connection.error_path.as_ref());

        return finish(connection, reader, ReplyEnd::Failed(masked(error, key)));
    }

    let end = match connection.answer {
        AnswerMode::Whole => match read_all(&mut response, raw, pieces).await {
            Read::Bytes(body) => match reader.whole(&body) {
                Ok(()) => {
                    // The whole text is one piece.
                    if !reader.text.is_empty() {
                        let _ = pieces.send(reader.text.clone()).await;
                    }

                    ReplyEnd::Finished(reader.stop.unwrap_or(StopReason::Complete))
                }
                Err(error) => ReplyEnd::Failed(error),
            },
            Read::Cancelled => ReplyEnd::Cancelled,
            Read::Failed(error) => ReplyEnd::Failed(error),
        },
        AnswerMode::Stream => stream(&mut response, &mut reader, pieces, raw).await,
    };

    let end = match end {
        ReplyEnd::Failed(error) => ReplyEnd::Failed(masked(error, key)),
        other => other,
    };

    finish(connection, reader, end)
}

/// Reads a streamed reply up to its end, passing each piece on.
async fn stream(
    response: &mut Response,
    reader: &mut AnswerReader<'_>,
    pieces: &mpsc::Sender<String>,
    raw: &mut Option<Raw>,
) -> ReplyEnd {
    let mut decoder = SseDecoder::default();

    loop {
        let chunk = tokio::select! {
            chunk = response.chunk() => chunk,
            () = pieces.closed() => return ReplyEnd::Cancelled,
        };

        let bytes = match chunk {
            Ok(Some(bytes)) => bytes,
            Ok(None) => break,
            Err(error) => return ReplyEnd::Failed(AiError::NoConnection(explain(&error))),
        };

        Raw::keep(raw, &bytes);

        for event in decoder.push(&bytes) {
            match reader.event(&event) {
                // A receiver that is gone is noticed by the select
                // above.
                Ok(Some(piece)) => {
                    let _ = pieces.send(piece).await;
                }
                Ok(None) => {}
                Err(error) => return ReplyEnd::Failed(error),
            }

            if reader.done {
                return ReplyEnd::Finished(reader.stop.unwrap_or(StopReason::Complete));
            }
        }

        if reader.text.len() > MAX_ANSWER {
            return ReplyEnd::Failed(AiError::UnexpectedAnswer("the reply is larger than 8 MiB".to_owned()));
        }
    }

    if reader.complete() {
        ReplyEnd::Finished(reader.stop.unwrap_or(StopReason::Complete))
    } else {
        ReplyEnd::Failed(AiError::NoConnection("the reply broke off".to_owned()))
    }
}

/// What reading a whole body gave.
enum Read {
    Bytes(Vec<u8>),
    Cancelled,
    Failed(AiError),
}

/// Reads a whole body, at most [`MAX_ANSWER`] bytes; stops when the
/// receiver of `pieces` is dropped.
async fn read_all(response: &mut Response, raw: &mut Option<Raw>, pieces: &mpsc::Sender<String>) -> Read {
    let mut body = Vec::new();

    loop {
        let chunk = tokio::select! {
            chunk = response.chunk() => chunk,
            () = pieces.closed() => return Read::Cancelled,
        };

        match chunk {
            Ok(Some(bytes)) => {
                Raw::keep(raw, &bytes);

                if body.len() + bytes.len() > MAX_ANSWER {
                    return Read::Failed(AiError::UnexpectedAnswer("the answer is larger than 8 MiB".to_owned()));
                }

                body.extend_from_slice(&bytes);
            }
            Ok(None) => return Read::Bytes(body),
            Err(error) => return Read::Failed(AiError::NoConnection(explain(&error))),
        }
    }
}

/// The HTTP client of a connection, with the rules at the top of
/// this file. Made for every request: a connection may have a CA
/// certificate of its own, and a chat makes few requests.
fn http_client(connection: &Checked) -> Result<Client, AiError> {
    let mut builder = Client::builder()
        .https_only(connection.url.starts_with("https://"))
        .tls_version_min(Version::TLS_1_3)
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(match connection.answer {
            AnswerMode::Stream => STREAM_SILENCE,
            AnswerMode::Whole => WHOLE_SILENCE,
        })
        .user_agent(concat!("io-master/", env!("CARGO_PKG_VERSION")));

    if let Some(certificates) = &connection.ca_certificates {
        builder = builder.tls_certs_only(certificates.iter().cloned());
    }

    builder
        .build()
        .map_err(|error| AiError::NoConnection(format!("the connection cannot be prepared: {}", explain(&error))))
}

/// Sends the request. Its headers are checked already
/// (`connection.rs`); one that still does not fit HTTP is an error,
/// not a panic.
async fn send(http: &Client, request: Request) -> Result<Response, AiError> {
    let mut builder = http.post(&request.url);

    for (name, value) in &request.headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| AiError::UnexpectedAnswer(format!("the header name {name} cannot be sent")))?;
        let mut value = HeaderValue::from_str(value)
            .map_err(|_| AiError::UnexpectedAnswer(format!("the header {name} cannot be sent")))?;

        // Kept out of any debug output of the HTTP library.
        value.set_sensitive(true);
        builder = builder.header(name, value);
    }

    if !request.headers.iter().any(|(name, _)| name == "content-type") {
        builder = builder.header("content-type", "application/json");
    }

    builder.body(request.body.to_string()).send().await.map_err(|error| AiError::NoConnection(explain(&error)))
}

/// The reply from what was read and how it ended. Without a model
/// named by the service, the model of the connection wrote it.
fn finish(connection: &Checked, reader: AnswerReader<'_>, end: ReplyEnd) -> Reply {
    let model = reader.model.or_else(|| (!connection.model.is_empty()).then(|| connection.model.clone()));

    Reply { text: reader.text, model, input_tokens: reader.input_tokens, output_tokens: reader.output_tokens, end }
}

/// The text with every occurrence of the key replaced by [`MASK`].
/// A service may repeat a header in an error (a debugging proxy, for
/// example); the key must still not reach the page.
fn mask(text: &str, key: Option<&ApiKey>) -> String {
    match key {
        Some(key) if !key.expose().is_empty() => text.replace(key.expose(), MASK),
        _ => text.to_owned(),
    }
}

/// The error with the key masked in its explanation.
fn masked(error: AiError, key: Option<&ApiKey>) -> AiError {
    let hide = |text: String| mask(&text, key);

    match error {
        AiError::KeyRejected(text) => AiError::KeyRejected(hide(text)),
        AiError::NotFound(text) => AiError::NotFound(hide(text)),
        AiError::LimitReached(text) => AiError::LimitReached(hide(text)),
        AiError::Rejected(status, text) => AiError::Rejected(status, hide(text)),
        AiError::ServiceFailed(status, text) => AiError::ServiceFailed(status, hide(text)),
        AiError::Reported(text) => AiError::Reported(hide(text)),
        AiError::NoConnection(text) => AiError::NoConnection(hide(text)),
        AiError::UnexpectedAnswer(text) => AiError::UnexpectedAnswer(hide(text)),
        AiError::NoKey => AiError::NoKey,
    }
}

/// The explanation of a failed connection: the error and its
/// causes ("error sending request" alone says little).
fn explain(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "no answer in time".to_owned();
    }

    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);

    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }

    text
}

#[cfg(test)]
mod tests {
    //! Unit tests of `client.rs`, against the fake service of
    //! `test_support.rs`: real TLS (or plain HTTP) on 127.0.0.1, no
    //! network.

    use super::*;

    use std::time::Duration;

    use serde_json::json;

    use crate::ai::api::test_support::{
        FakeAnswer, FakeService, checked, completions, completions_events, data, messages_events, messages_format, sse,
    };
    use crate::model_old::ai::Connection;

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
        assert_eq!(request.body["system"], crate::model_old::ai::DEFAULT_SYSTEM_PROMPT);
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
}
