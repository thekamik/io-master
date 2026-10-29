//! The connection to an AI service: one request, the answer read while it
//! streams, its pieces passed on into a channel.
//!
//! **The connection protects the key and the conversation:**
//!
//! - the address is `https://`, or `http://` only on this computer (a local
//!   model); TLS 1.3 or newer, the certificate verified against the trusted
//!   certificates of the system, or only against the CA certificate of the
//!   integration ([`check_address`], [`http_client`]);
//! - redirects are never followed, so the key and the conversation go to
//!   the address of the integration and nowhere else;
//! - proxies of the environment (`HTTPS_PROXY`) are not used;
//! - the key is marked "sensitive" in its header, and masked in every text
//!   taken from an answer ([`masked`]).
//!
//! **An answer can be stopped by the caller**: [`send`] passes the pieces
//! of the answer into a channel and stops as soon as the receiving end of
//! the channel is dropped. What was written up to then is kept.
//!
//! ```text
//! send ──► exchange ──► http_client, the POST ──► an answer that is no success ──► read_error ──► AiError
//!                                              └► read_stream ──► SseDecoder ──► read_event (openai.rs, anthropic.rs)
//!                                                                                  └► the pieces, into the channel
//! ```

use std::time::Duration;

use reqwest::header::HeaderValue;
use reqwest::redirect::Policy;
use reqwest::tls::Version;
use reqwest::{Certificate, Client, Response, Url};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

use crate::ai::error::{AiError, from_status};
use crate::ai::reading::Reading;
use crate::ai::sse::{SseDecoder, SseEvent};
use crate::ai::{anthropic, openai};
use crate::model::ai::integration::Protocol;

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long a stream may stay silent. A local model on a slow computer may
/// think long before its first piece, so this is generous; a longer silence
/// means a broken connection.
const STREAM_SILENCE: Duration = Duration::from_secs(300);

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

/// What stands in place of the key where a text could hold it.
pub const MASK: &str = "••••••••";

/// A request ready to be sent.
pub struct Request {
    /// The address of the service.
    pub url: String,
    /// The CA certificate (PEM) that alone is trusted; `None`: the trusted
    /// certificates of the system.
    pub ca_certificate: Option<String>,
    /// The way the program speaks with the service.
    pub protocol: Protocol,
    /// The API key; `None` for a local service without one.
    pub key: Option<Zeroizing<String>>,
    /// The body (JSON); the key is never in it.
    pub body: String,
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

/// Refuses an address the program does not send to: one that is no URL,
/// one that is neither `https://` nor `http://` on this computer.
///
/// Called by: `ask.rs` ([`crate::ai::ask::prepare`]); the settings of the
/// AI (an integration saved).
pub fn check_address(url: &str) -> Result<(), AiError> {
    let parsed = Url::parse(url).map_err(|error| AiError::Address(format!("{url}: {error}")))?;
    let local = matches!(parsed.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));

    ensure!(
        parsed.scheme() == "https" || (parsed.scheme() == "http" && local),
        AiError::Address("only https://, or http:// on this computer (127.0.0.1, localhost)".to_owned())
    );
    ensure!(
        parsed.username().is_empty() && parsed.password().is_none(),
        AiError::Address("a password in the address".to_owned())
    );

    Ok(())
}

/// The certificates of a CA certificate in PEM; refused when it holds none
/// or is no PEM.
///
/// Called by: [`http_client`]; the settings of the AI (an integration
/// saved, through [`check_certificate`]).
fn certificates(pem: &str) -> Result<Vec<Certificate>, AiError> {
    let certificates = Certificate::from_pem_bundle(pem.as_bytes())
        .map_err(|error| AiError::Address(format!("the CA certificate: {error}")))?;

    ensure!(!certificates.is_empty(), AiError::Address("the CA certificate holds no certificate".to_owned()));

    Ok(certificates)
}

/// Refuses a CA certificate the connection could not use.
///
/// Called by: the settings of the AI (an integration saved).
/// Calls: [`certificates`].
pub fn check_certificate(pem: &str) -> Result<(), AiError> {
    certificates(pem)?;

    Ok(())
}

/// Sends the request and reads the answer, passing every piece of its text
/// into `pieces`; stops when the receiver of `pieces` is dropped. Never
/// fails as a whole: how it ended is in [`Received`], next to what was
/// written until then.
///
/// Called by: `ask.rs` ([`crate::ai::ask::send`]).
/// Calls: [`exchange`], [`masked`].
pub async fn send(request: &Request, pieces: &mpsc::Sender<String>) -> Received {
    let mut reading = Reading::default();

    match exchange(request, pieces, &mut reading).await {
        Ok(stopped) => Received { reading, stopped, error: None },
        Err(error) => {
            Received { reading, stopped: false, error: Some(masked(error, request.key.as_deref().map(String::as_str))) }
        }
    }
}

/// The work of [`send`]; gives whether the caller stopped the answer.
///
/// Called by: [`send`].
/// Calls: [`http_client`], [`post`], [`read_error`], [`read_stream`].
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

    match read_stream(&mut response, request.protocol, pieces, reading).await? {
        StreamEnd::Done => Ok(false),
        StreamEnd::Stopped => Ok(true),
        StreamEnd::Closed => {
            ensure!(reading.ending.is_some(), AiError::NoConnection("the answer broke off".to_owned()));
            Ok(false)
        }
    }
}

/// The HTTP client of a request, with the rules at the top of this file.
/// Made for every request: an integration may have a CA certificate of its
/// own, and the program asks few questions.
///
/// Called by: [`exchange`].
fn http_client(request: &Request) -> Result<Client, AiError> {
    let mut builder = Client::builder()
        .https_only(request.url.starts_with("https://"))
        .tls_version_min(Version::TLS_1_3)
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(STREAM_SILENCE)
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
/// Called by: [`exchange`].
async fn post(client: &Client, request: &Request) -> Result<Response, AiError> {
    let mut headers: Vec<(&'static str, Zeroizing<String>)> = Vec::with_capacity(2);

    match (request.protocol, request.key.as_deref()) {
        (Protocol::OpenAiChat, Some(key)) => headers.push(openai::key_header(key)),
        (Protocol::OpenAiChat, None) => {}
        (Protocol::AnthropicMessages, Some(key)) => headers.extend(anthropic::key_headers(key)),
        (Protocol::AnthropicMessages, None) => return Err(AiError::NoKey),
    }

    let mut builder = client.post(&request.url).header("content-type", "application/json");

    for (name, value) in &headers {
        let mut value = HeaderValue::from_str(value)
            .map_err(|_| AiError::Address(format!("the header {name} cannot be sent (a character of the key)")))?;

        value.set_sensitive(true);
        builder = builder.header(*name, value);
    }

    builder.body(request.body.clone()).send().await.map_err(|error| AiError::NoConnection(explain(&error)))
}

/// Reads one event of the answer with the reader of its protocol.
///
/// Called by: [`read_stream`].
fn read_event(protocol: Protocol, event: &SseEvent, reading: &mut Reading) -> Result<Option<String>, AiError> {
    match protocol {
        Protocol::OpenAiChat => openai::read_event(event, reading),
        Protocol::AnthropicMessages => anthropic::read_event(event, reading),
    }
}

/// Reads a streamed answer up to its end, passing each piece of text on.
///
/// Called by: [`exchange`].
/// Calls: `SseDecoder::push`, [`read_event`].
async fn read_stream(
    response: &mut Response,
    protocol: Protocol,
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
            if let Some(piece) = read_event(protocol, &event, reading)?
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
/// bytes; what cannot be read is left out (the status still tells the
/// kind).
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

/// The text with every occurrence of the key replaced by [`MASK`]: a
/// service may repeat a header in an error.
///
/// Called by: [`masked`].
fn mask(text: String, key: Option<&str>) -> String {
    match key {
        Some(key) if !key.is_empty() && text.contains(key) => text.replace(key, MASK),
        _ => text,
    }
}

/// The error with the key masked in its explanation.
///
/// Called by: [`send`].
/// Calls: [`mask`].
fn masked(error: AiError, key: Option<&str>) -> AiError {
    match error {
        AiError::Address(text) => AiError::Address(mask(text, key)),
        AiError::KeyRejected(text) => AiError::KeyRejected(mask(text, key)),
        AiError::NotFound(text) => AiError::NotFound(mask(text, key)),
        AiError::LimitReached(text) => AiError::LimitReached(mask(text, key)),
        AiError::Rejected(status, text) => AiError::Rejected(status, mask(text, key)),
        AiError::ServiceFailed(status, text) => AiError::ServiceFailed(status, mask(text, key)),
        AiError::Reported(text) => AiError::Reported(mask(text, key)),
        AiError::NoConnection(text) => AiError::NoConnection(mask(text, key)),
        AiError::UnexpectedAnswer(text) => AiError::UnexpectedAnswer(mask(text, key)),
        other @ (AiError::NoKey | AiError::NoQuestion | AiError::TooLong { .. } | AiError::Db(_)) => other,
    }
}

/// The explanation of a failed connection: the error and its causes ("error
/// sending request" alone says little).
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

    use serde_json::json;

    use crate::model::ai::journal::Ending;
    use crate::test_support::ai_service::{FakeAnswer, FakeService, anthropic_events, openai_events, sse};

    const KEY: &str = "sk-test-secret-key";

    fn request(service: &FakeService, protocol: Protocol) -> Request {
        Request {
            url: format!("{}/v1/endpoint", service.url),
            ca_certificate: service.ca_pem.clone(),
            protocol,
            key: Some(Zeroizing::new(KEY.to_owned())),
            body: json!({"model": "m", "messages": [{"role": "user", "content": "Hi"}]}).to_string(),
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
        let received = send(request, &sender).await;

        drop(sender);
        (received, collect.await.unwrap())
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
        assert!(check_address("http://example.com/v1/chat/completions").is_err());
        assert!(check_address("https://user:pw@example.com/v1").is_err());
        assert!(check_address("https://api.anthropic.com/v1/messages").is_ok());
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

        let received = tokio::time::timeout(Duration::from_secs(10), sending).await.unwrap().unwrap();

        assert!(received.stopped);
        assert_eq!(received.reading.text, "First");
    }
}
