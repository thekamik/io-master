//! The connection to an AI service: one request, the answer read while it
//! streams, its pieces passed on into a channel.
//!
//! **The connection protects the key and the conversation:**
//!
//! - the address is `https://`, or `http://` only on this computer (a local
//!   model); TLS 1.3 or newer, the certificate verified against the trusted
//!   certificates of the system, or only against the CA certificate of the
//!   integration (`integration_rules/address.rs`, [`http_client`]);
//! - redirects are never followed, so the key and the conversation go to
//!   the address of the integration and nowhere else;
//! - proxies of the environment (`HTTPS_PROXY`) are not used;
//! - the key is marked "sensitive" in its header, and masked in every text
//!   taken from an answer (`objects/ai/api_key/api_key_ai.rs`).
//!
//! **An answer can be stopped by the caller**: [`send`] passes the pieces
//! of the answer into a channel and stops as soon as the receiving end of
//! the channel is dropped. What was written up to then is kept.
//!
//! ```text
//! send ──► exchange ──► http_client, the POST ──► an answer that is no success ──► read_error ──► AiError
//!                                              ├► read_stream ──► SseDecoder ──► protocol.rs: read_event
//!                                              │                                   └► the pieces, into the channel
//!                                              └► read_whole_answer (a custom service that answers with one
//!                                                 JSON document) ──► custom_format_ai.rs: read_whole
//! ```

use std::time::Duration;

use reqwest::header::HeaderValue;
use reqwest::redirect::Policy;
use reqwest::tls::Version;
use reqwest::{Client, Response};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

use crate::objects::ai::api_key::api_key_ai::masked;
use crate::objects::ai::custom_format::custom_format_ai::{CustomExchange, read_whole};
use crate::objects::ai::integration::integration_ai::protocol::{key_headers, read_event};
use crate::objects::ai::integration::integration_ai::reading::Reading;
use crate::objects::ai::integration::integration_ai::sse::SseDecoder;
use crate::objects::ai::integration::integration_model::Protocol;
use crate::objects::ai::integration::integration_rules::address::{certificates, check_address};
use crate::objects::ai::journal::journal_model::Ending;
use crate::objects::ai_error::{AiError, from_status};

/// The most bytes of a streamed answer: far more than any answer (its
/// events carry a few bytes of text each); it stops a broken service from
/// filling the memory.
const MAX_STREAM: usize = 32 * 1024 * 1024;

/// The most pieces of the stream read: every piece has at least one byte.
const MAX_CHUNKS: usize = MAX_STREAM;

/// The most bytes read of an answer that is no success: enough for its
/// explanation.
const MAX_ERROR: usize = 64 * 1024;

/// The most causes of a failed connection named in its explanation.
const MAX_CAUSES: usize = 8;

/// A request ready to be sent.
pub struct Request {
    /// The address of the service.
    pub url: String,
    /// The CA certificate (PEM) that alone is trusted; `None`: the trusted
    /// certificates of the system.
    pub ca_certificate: Option<String>,
    /// The way the program speaks with the service.
    pub protocol: Protocol,
    /// How a custom service is spoken with (the header of its key, how its
    /// answer comes and where it is read); given for `Protocol::Custom`.
    pub custom: Option<CustomExchange>,
    /// The API key; `None` for a local service without one.
    pub key: Option<Zeroizing<String>>,
    /// The body (JSON); the key is never in it.
    pub body: String,
    /// How long the connection may take to open
    /// (`Integration::connect_timeout_seconds`).
    pub connect_timeout: Duration,
    /// How long the stream may stay silent
    /// (`Integration::stream_silence_seconds`).
    pub stream_silence: Duration,
}

/// What came back: the answer read, complete or not.
pub struct Received {
    /// What the answer said.
    pub reading: Reading,
    /// The caller stopped the answer.
    pub stopped: bool,
    /// Why the request or the answer failed, the key masked.
    pub error: Option<AiError>,
}

/// How a stream ended.
enum StreamEnd {
    /// The service said it is over.
    Done,
    /// The service closed the connection.
    Closed,
    /// The caller dropped the receiver of the pieces.
    Stopped,
}

/// Sends the request and reads the answer, passing every piece of its text
/// into `pieces`; stops when the receiver of `pieces` is dropped.
///
/// Input: `request`, the request prepared (`question.rs`: `prepare`), its
/// address checked again here; `pieces`, where each piece of text goes.
/// Output: never fails as a whole: how it ended is in [`Received`], next to
/// what was written until then (`Received { reading, stopped: false, error:
/// None }`), its error with the key masked.
///
/// Called by: `objects/ai/integration/integration_ai/sending.rs`: `send`.
/// Calls: [`exchange`]; `objects/ai/api_key/api_key_ai.rs`: `masked`.
pub async fn send(request: &Request, pieces: &mpsc::Sender<String>) -> Received {
    let mut reading = Reading::default();

    match exchange(request, pieces, &mut reading).await {
        Ok(stopped) => Received { reading, stopped, error: None },
        Err(error) => {
            Received { reading, stopped: false, error: Some(masked(error, request.key.as_deref().map(String::as_str))) }
        }
    }
}

/// The work of [`send`]: the address checked, the request posted, the
/// answer read as a stream.
///
/// Input: as [`send`]; `reading`, what was read of the answer so far.
/// Output: whether the caller stopped the answer (`false`); an error for an
/// address refused, a connection failed, a status that is no success, a
/// broken or unreadable stream.
///
/// Called by: [`send`].
/// Calls: `objects/ai/integration/integration_rules/address.rs`: `check_address`; [`http_client`], [`post`],
/// [`read_error`], [`read_whole_answer`], [`read_stream`]; `objects/ai_error.rs`: `from_status`.
async fn exchange(request: &Request, pieces: &mpsc::Sender<String>, reading: &mut Reading) -> Result<bool, AiError> {
    check_address(&request.url)?;

    let client = http_client(request)?;
    let sent = tokio::select! {
        sent = post(&client, request) => sent,
        () = pieces.closed() => return Ok(true),
    };
    let mut response = sent?;
    let status = response.status();

    if !status.is_success() {
        let body = read_error(&mut response).await;

        return Err(from_status(status.as_u16(), &body));
    }

    // A custom service may give its answer whole, as one JSON document.
    if let Some(custom) = request.custom.as_ref().filter(|custom| !custom.events) {
        return read_whole_answer(&mut response, custom, pieces, reading).await;
    }

    match read_stream(&mut response, request, pieces, reading).await? {
        StreamEnd::Done => Ok(false),
        StreamEnd::Stopped => Ok(true),
        // The stream of a custom service has no mark of its end the program
        // could ask for: one that the connection closed in order is whole.
        StreamEnd::Closed if request.custom.is_some() => {
            reading.ending.get_or_insert(Ending::Complete);
            Ok(false)
        }
        StreamEnd::Closed => {
            ensure!(reading.ending.is_some(), AiError::NoConnection("the answer broke off".to_owned()));
            Ok(false)
        }
    }
}

/// The HTTP client of a request, with the rules at the top of this file and
/// the waits of its integration.
/// Made for every request: an integration may have a CA certificate of its
/// own, and the program asks few questions.
///
/// Input: `request`, the request; its address, CA certificate and waits are
/// used.
/// Output: the client; an error for a certificate refused, or when the
/// library cannot make it.
///
/// Called by: [`exchange`].
/// Calls: `objects/ai/integration/integration_rules/address.rs`: `certificates`; [`explain`].
fn http_client(request: &Request) -> Result<Client, AiError> {
    let mut builder = Client::builder()
        .https_only(request.url.starts_with("https://"))
        .tls_version_min(Version::TLS_1_3)
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(request.connect_timeout)
        .read_timeout(request.stream_silence)
        .user_agent(concat!("io-master/", env!("CARGO_PKG_VERSION")));

    if let Some(pem) = &request.ca_certificate {
        builder = builder.tls_certs_only(certificates(pem)?);
    }

    builder
        .build()
        .map_err(|error| AiError::NoConnection(format!("the connection cannot be prepared: {}", explain(&error))))
}

/// Sends the request: its headers (the key marked sensitive, so that it
/// stays out of any debug output of the library) and its body.
///
/// Input: `client`, made by [`http_client`]; `request`, the request.
/// Output: the response, its body not read yet; `AiError::Address` for a key
/// a header cannot carry, `AiError::NoConnection` when sending fails.
///
/// Called by: [`exchange`].
/// Calls: `objects/ai/integration/integration_ai/protocol.rs`: `key_headers`; [`explain`].
async fn post(client: &Client, request: &Request) -> Result<Response, AiError> {
    let headers = key_headers(request.protocol, request.key.as_deref().map(String::as_str), request.custom.as_ref())?;
    let mut builder = client.post(&request.url).header("content-type", "application/json");

    for (name, value) in &headers {
        let mut value = HeaderValue::from_str(value)
            .map_err(|_| AiError::Address(format!("the header {name} cannot be sent (a character of the key)")))?;

        value.set_sensitive(true);
        builder = builder.header(name.as_str(), value);
    }

    builder.body(request.body.clone()).send().await.map_err(|error| AiError::NoConnection(explain(&error)))
}

/// Reads the answer of a custom service that comes whole: every byte of its
/// body up to [`MAX_STREAM`], then the one JSON document it is, read by the
/// format. Nothing of it is cut: a body over the limit is an error.
///
/// Input: `response`, a response with a success status; `custom`, how the
/// service is spoken with; `pieces`, where the text goes, in one piece;
/// `reading`, empty.
/// Output: `true` when the caller stopped the answer before it was whole
/// (nothing is read then), `false` when it was read; an error for a broken
/// connection, a body over [`MAX_STREAM`], the errors of the reading.
///
/// Called by: [`exchange`].
/// Calls: `objects/ai/custom_format/custom_format_ai.rs`: `read_whole`; [`explain`].
async fn read_whole_answer(
    response: &mut Response,
    custom: &CustomExchange,
    pieces: &mpsc::Sender<String>,
    reading: &mut Reading,
) -> Result<bool, AiError> {
    let mut body = Vec::new();

    for _ in 0..MAX_CHUNKS {
        let chunk = tokio::select! {
            chunk = response.chunk() => chunk,
            () = pieces.closed() => return Ok(true),
        };
        let Some(bytes) = chunk.map_err(|error| AiError::NoConnection(explain(&error)))? else {
            let text = read_whole(&body, custom, reading)?;

            // A receiver gone by now is a caller that stopped the answer.
            return Ok(pieces.send(text).await.is_err());
        };

        ensure!(body.len() + bytes.len() <= MAX_STREAM, AiError::UnexpectedAnswer("an answer over 32 MiB".to_owned()));
        body.extend_from_slice(&bytes);
    }

    Err(AiError::UnexpectedAnswer("an answer in too many pieces".to_owned()))
}

/// Reads a streamed answer up to its end, passing each piece of text on.
///
/// Input: `response`, a response with a success status; `request`, the
/// request it answers (its protocol, and its custom exchange when it has
/// one); `pieces`, where each piece goes; `reading`, what was read so far.
/// Output: how the stream ended (`StreamEnd::Done`); an error for a broken
/// connection, an answer over [`MAX_STREAM`] or in more than [`MAX_CHUNKS`]
/// pieces, an event that cannot be read, an error the service reports.
///
/// Called by: [`exchange`].
/// Calls: `objects/ai/integration/integration_ai/sse.rs`: `SseDecoder::push`;
/// `objects/ai/integration/integration_ai/protocol.rs`: `read_event`; [`explain`].
async fn read_stream(
    response: &mut Response,
    request: &Request,
    pieces: &mpsc::Sender<String>,
    reading: &mut Reading,
) -> Result<StreamEnd, AiError> {
    let mut decoder = SseDecoder::default();
    let mut received = 0;

    for _ in 0..MAX_CHUNKS {
        let chunk = tokio::select! {
            chunk = response.chunk() => chunk,
            () = pieces.closed() => return Ok(StreamEnd::Stopped),
        };
        let Some(bytes) = chunk.map_err(|error| AiError::NoConnection(explain(&error)))? else {
            return Ok(StreamEnd::Closed);
        };

        received += bytes.len();
        ensure!(received <= MAX_STREAM, AiError::UnexpectedAnswer("an answer over 32 MiB".to_owned()));

        for event in decoder.push(&bytes)? {
            if let Some(piece) = read_event(request.protocol, &event, reading, request.custom.as_ref())?
                && pieces.send(piece).await.is_err()
            {
                return Ok(StreamEnd::Stopped);
            }

            if reading.done {
                return Ok(StreamEnd::Done);
            }
        }
    }

    Err(AiError::UnexpectedAnswer("an answer in too many pieces".to_owned()))
}

/// Reads the body of an answer that is no success, at most [`MAX_ERROR`]
/// bytes.
///
/// Input: `response`, a response whose status is no success.
/// Output: the bytes read (`b"{\"error\": ...}"`); what cannot be read is
/// left out, as the status still tells the kind.
///
/// Called by: [`exchange`].
async fn read_error(response: &mut Response) -> Vec<u8> {
    let mut body = Vec::new();

    for _ in 0..MAX_ERROR {
        match response.chunk().await {
            Ok(Some(bytes)) if body.len() + bytes.len() <= MAX_ERROR => body.extend_from_slice(&bytes),
            _ => break,
        }
    }

    body
}

/// The explanation of a failed connection: the error and its causes ("error
/// sending request" alone says little).
///
/// Input: `error`, the error of the library.
/// Output: the text ("error sending request: connection refused"); "no
/// answer in time" for a timeout.
///
/// Called by: [`http_client`], [`post`], [`read_stream`].
fn explain(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "no answer in time".to_owned();
    }

    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);

    for _ in 0..MAX_CAUSES {
        let Some(cause) = source else { break };

        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }

    text
}

#[cfg(test)]
mod tests {
    //! Tests of the connection, against the fake service of
    //! `test_support/ai_service.rs`: real TLS (or plain HTTP) on 127.0.0.1,
    //! no network.

    use super::*;

    use crate::objects::error::DomainError;

    use serde_json::json;

    use crate::objects::ai::api_key::api_key_ai::MASK;
    use crate::objects::ai::chat::chat_model::Speaker;
    use crate::objects::ai::custom_format::custom_format_ai::{custom_body, custom_exchange};
    use crate::objects::ai::custom_format::custom_format_model::{CustomFormat, ResponseKind};
    use crate::objects::ai::integration::integration_ai::turns::Turn;
    use crate::objects::ai::journal::journal_model::Ending;
    use crate::test_support::ai_service::{FakeAnswer, FakeService, anthropic_events, openai_events, sse};
    use crate::test_support::database::custom_format;

    const KEY: &str = "sk-test-secret-key";

    fn request(service: &FakeService, protocol: Protocol) -> Request {
        Request {
            url: format!("{}/v1/endpoint", service.url),
            ca_certificate: service.ca_pem.clone(),
            protocol,
            custom: None,
            key: Some(Zeroizing::new(KEY.to_owned())),
            body: json!({"model": "m", "messages": [{"role": "user", "content": "Hi"}]}).to_string(),
            connect_timeout: Duration::from_secs(15),
            stream_silence: Duration::from_secs(300),
        }
    }

    /// Sends a request and collects the pieces passed on meanwhile.
    async fn send_collecting(request: &Request) -> (Received, Vec<String>) {
        let (sender, mut receiver) = mpsc::channel(16);
        let collect = tokio::spawn(async move {
            let mut pieces = Vec::new();

            while let Some(piece) = receiver.recv().await {
                pieces.push(piece);
            }

            pieces
        });
        let outcome = send(request, &sender).await;

        drop(sender);
        (outcome, collect.await.unwrap())
    }

    #[tokio::test]
    async fn an_openai_answer_arrives_in_pieces_with_the_key_in_its_header() {
        let service = FakeService::start(FakeAnswer::Stream(openai_events(&["Hel", "lo"]))).await;
        let (received, pieces) = send_collecting(&request(&service, Protocol::OpenAiChat)).await;

        assert_eq!(pieces, ["Hel", "lo"]);
        assert_eq!(received.reading.text, "Hello");
        assert_eq!(received.reading.ending, Some(Ending::Complete));
        assert!(received.error.is_none() && !received.stopped);

        let sent = &service.received()[0];

        assert!(sent.head.starts_with("POST /v1/endpoint "));
        assert_eq!(sent.header("authorization"), Some(format!("Bearer {KEY}").as_str()));
        assert_eq!(sent.header("content-type"), Some("application/json"));
        assert!(!sent.body.to_string().contains(KEY));
    }

    /// A request to a custom service that speaks by `format`, the key in
    /// the header "X-Api-Key" after "Token ".
    fn custom_request(service: &FakeService, format: &CustomFormat) -> Request {
        let format =
            CustomFormat { key_header: "X-Api-Key".to_owned(), key_prefix: "Token ".to_owned(), ..format.clone() };
        let turns = [Turn { speaker: Speaker::User, text: "Hi".to_owned() }];

        Request {
            custom: Some(custom_exchange(&format).unwrap()),
            body: custom_body(&format, "institute-7", "You assess.", &turns, 100, None).unwrap().to_string(),
            ..request(service, Protocol::Custom)
        }
    }

    #[tokio::test]
    async fn a_custom_service_is_asked_by_its_request_and_its_whole_answer_read_by_its_response() {
        let answer = r#"{"response": "PASS, with a long analysis.", "trace": {"id": 7}}"#;
        let service = FakeService::start(FakeAnswer::Json(200, answer.to_owned())).await;
        let (received, pieces) = send_collecting(&custom_request(&service, &custom_format())).await;

        assert!(received.error.is_none() && !received.stopped);
        assert_eq!(pieces, ["PASS, with a long analysis."]);
        assert_eq!(received.reading.text, "PASS, with a long analysis.");
        assert_eq!(received.reading.ending, Some(Ending::Complete));

        // The question went in the shape of the format, the key only in its header.
        let sent = &service.received()[0];

        assert_eq!(sent.body, json!({"engine": "institute-7", "system": "You assess.", "prompt": "Hi"}));
        assert_eq!(sent.header("x-api-key"), Some(format!("Token {KEY}").as_str()));
        assert_eq!(sent.header("authorization"), None);
        assert!(!sent.body.to_string().contains(KEY));
    }

    #[tokio::test]
    async fn a_custom_answer_of_a_megabyte_is_read_whole() {
        let long = "analysis ".repeat(120_000);
        let service = FakeService::start(FakeAnswer::Json(200, json!({"response": long}).to_string())).await;
        let (received, _) = send_collecting(&custom_request(&service, &custom_format())).await;

        assert!(received.error.is_none());
        assert_eq!(received.reading.text.len(), long.len());
    }

    #[tokio::test]
    async fn a_custom_stream_is_read_event_by_event_to_its_end() {
        let format = CustomFormat {
            response: r#"{"delta": "<response>", "fault": "<error>"}"#.to_owned(),
            response_kind: ResponseKind::Events,
            ..custom_format()
        };
        let events = vec![
            sse("", &json!({"delta": "PA"})),
            sse("", &json!({"status": "busy"})),
            sse("", &json!({"delta": "SS"})),
        ];
        let service = FakeService::start(FakeAnswer::Stream(events)).await;
        let (received, pieces) = send_collecting(&custom_request(&service, &format)).await;

        // The stream has no mark of its end: closed in order, it is whole.
        assert!(received.error.is_none() && !received.stopped);
        assert_eq!(pieces, ["PA", "SS"]);
        assert_eq!((received.reading.text.as_str(), received.reading.ending), ("PASS", Some(Ending::Complete)));

        let failing = vec![sse("", &json!({"delta": "PA"})), sse("", &json!({"fault": "the model fell over"}))];
        let service = FakeService::start(FakeAnswer::Stream(failing)).await;
        let (received, _) = send_collecting(&custom_request(&service, &format)).await;

        assert!(matches!(received.error, Some(AiError::Reported(said)) if said == "the model fell over"));
        assert_eq!(received.reading.text, "PA");
    }

    #[tokio::test]
    async fn a_custom_answer_that_does_not_fit_its_format_is_told() {
        for body in ["<html>busy</html>", r#"{"other": "shape"}"#, r#"{"response": 7}"#] {
            let service = FakeService::start(FakeAnswer::Json(200, body.to_owned())).await;
            let (received, pieces) = send_collecting(&custom_request(&service, &custom_format())).await;

            assert!(matches!(received.error, Some(AiError::UnexpectedAnswer(_))), "{body}");
            assert!(pieces.is_empty() && received.reading.text.is_empty());
        }

        let service = FakeService::start(FakeAnswer::Json(503, r#"{"detail": "down"}"#.to_owned())).await;
        let (received, _) = send_collecting(&custom_request(&service, &custom_format())).await;

        assert!(matches!(received.error, Some(AiError::ServiceFailed(503, _))));
    }

    #[tokio::test]
    async fn an_anthropic_answer_sends_its_key_and_version() {
        let service = FakeService::start(FakeAnswer::Stream(anthropic_events(&["Hi ", "there"]))).await;
        let (received, _) = send_collecting(&request(&service, Protocol::AnthropicMessages)).await;

        assert_eq!(received.reading.text, "Hi there");
        assert_eq!(service.received()[0].header("x-api-key"), Some(KEY));
        assert_eq!(service.received()[0].header("anthropic-version"), Some("2026-10-21"));

        let without_key = Request { key: None, ..request(&service, Protocol::AnthropicMessages) };
        let (received, _) = send_collecting(&without_key).await;

        assert!(matches!(received.error, Some(AiError::NoKey)));
        assert_eq!(service.received().len(), 1);
    }

    #[tokio::test]
    async fn plain_http_only_on_this_computer() {
        let service = FakeService::start_plain(FakeAnswer::Stream(openai_events(&["local"]))).await;
        let local = Request { key: None, ..request(&service, Protocol::OpenAiChat) };
        let (received, _) = send_collecting(&local).await;

        assert_eq!(received.reading.text, "local");

        let remote = Request { url: "http://example.com/v1/chat/completions".to_owned(), ..local };
        let (received, _) = send_collecting(&remote).await;

        assert!(matches!(received.error, Some(AiError::Domain(DomainError::Refused(_)))));
    }

    #[tokio::test]
    async fn the_status_tells_the_kind_and_the_key_is_masked() {
        let body = json!({"error": {"message": format!("the header was Bearer {KEY}")}}).to_string();
        let service = FakeService::start(FakeAnswer::Json(401, body)).await;
        let (received, pieces) = send_collecting(&request(&service, Protocol::OpenAiChat)).await;

        assert!(pieces.is_empty());
        assert!(
            matches!(&received.error, Some(AiError::KeyRejected(text)) if text.contains(MASK) && !text.contains(KEY))
        );
    }

    #[tokio::test]
    async fn a_redirect_is_not_followed() {
        let service = FakeService::start(FakeAnswer::Json(307, String::new())).await;
        let (received, _) = send_collecting(&request(&service, Protocol::OpenAiChat)).await;

        assert!(matches!(received.error, Some(AiError::UnexpectedAnswer(_))));
        assert_eq!(service.received().len(), 1);
    }

    #[tokio::test]
    async fn tls_older_than_1_3_and_a_foreign_authority_are_refused() {
        let old = FakeService::start_tls12(FakeAnswer::Json(200, "{}".to_owned())).await;
        let (received, _) = send_collecting(&request(&old, Protocol::OpenAiChat)).await;

        assert!(matches!(received.error, Some(AiError::NoConnection(_))));

        let service = FakeService::start(FakeAnswer::Stream(openai_events(&["x"]))).await;
        let foreign = Request { ca_certificate: old.ca_pem.clone(), ..request(&service, Protocol::OpenAiChat) };
        let (received, _) = send_collecting(&foreign).await;

        assert!(matches!(received.error, Some(AiError::NoConnection(_))));

        // Without a certificate of its own, the system's are used, and they
        // do not know the authority of the test.
        let system = Request { ca_certificate: None, ..request(&service, Protocol::OpenAiChat) };
        let (received, _) = send_collecting(&system).await;

        assert!(matches!(received.error, Some(AiError::NoConnection(_))));
        assert!(old.received().is_empty() && service.received().is_empty());
    }

    #[tokio::test]
    async fn an_error_in_the_middle_keeps_the_text_and_a_broken_stream_fails() {
        let events = vec![
            sse("", &json!({"choices": [{"delta": {"content": "Part"}}]})),
            sse("", &json!({"error": {"message": "overloaded"}})),
        ];
        let service = FakeService::start(FakeAnswer::Stream(events)).await;
        let (received, _) = send_collecting(&request(&service, Protocol::OpenAiChat)).await;

        assert_eq!(received.reading.text, "Part");
        assert!(matches!(&received.error, Some(AiError::Reported(text)) if text == "overloaded"));

        let mut events = anthropic_events(&["Part"]);

        events.truncate(2);

        let service = FakeService::start(FakeAnswer::Stream(events)).await;
        let (received, _) = send_collecting(&request(&service, Protocol::AnthropicMessages)).await;

        assert_eq!(received.reading.text, "Part");
        assert!(matches!(received.error, Some(AiError::NoConnection(_))));
    }

    #[tokio::test]
    async fn dropping_the_receiver_stops_the_answer() {
        let events = vec![sse("", &json!({"choices": [{"delta": {"content": "First"}}]}))];
        let service = FakeService::start(FakeAnswer::StreamThenWait(events)).await;
        let request = request(&service, Protocol::OpenAiChat);
        let (sender, mut receiver) = mpsc::channel(16);
        let sending = tokio::spawn(async move { send(&request, &sender).await });

        // The first piece comes; then the reader goes away, as when the
        // user presses "Stop".
        assert_eq!(receiver.recv().await.as_deref(), Some("First"));
        drop(receiver);

        let outcome = tokio::time::timeout(Duration::from_secs(10), sending).await.unwrap().unwrap();

        assert!(outcome.stopped);
        assert_eq!(outcome.reading.text, "First");
    }
}
