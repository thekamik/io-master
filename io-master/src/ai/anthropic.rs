//! The connection to the Messages API of Anthropic: checking a key
//! and a model, and getting a reply to a conversation while it is
//! written.
//!
//! Rust has no official SDK of Anthropic, so the requests are made
//! by hand, in the documented HTTP form:
//!
//! ```text
//! POST https://api.anthropic.com/v1/messages
//! x-api-key: <the key>
//! anthropic-version: 2023-06-01
//! content-type: application/json
//!
//! {"model": "claude-opus-5-5", "max_tokens": 64000, "stream": true,
//!  "system": "...", "messages": [{"role": "user", "content": "..."}]}
//! ```
//!
//! **The connection protects the key** (as in v0.2): HTTPS only,
//! TLS 1.3 or newer, the certificate verified against the trusted
//! certificates of the system, and redirects never followed, so the
//! key is sent to `api.anthropic.com` and nowhere else. Proxies of
//! the environment (`HTTPS_PROXY`) are not used either.
//!
//! **A reply can be stopped by the caller**: [`Anthropic::reply`]
//! sends the pieces of the reply into a channel, and stops as soon
//! as the receiving end of the channel is dropped (the browser left
//! the page, or the evaluator pressed "Stop"). The text written up
//! to then is returned, so that it can be kept.

use std::time::Duration;

use reqwest::redirect::Policy;
use reqwest::tls::Version;
use reqwest::{Client, RequestBuilder};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::ai_error::AiError;
use super::api_key::ApiKey;
use super::model::Model;
use super::reply::{ReplyReader, StopReason};
use super::sse::SseDecoder;

/// Where the API of Anthropic is.
pub const SERVICE_URL: &str = "https://api.anthropic.com";

/// The address of the Messages API, after [`SERVICE_URL`].
const MESSAGES_PATH: &str = "/v1/messages";

/// The version of the API this program is written for.
const API_VERSION: &str = "2023-06-01";

/// The beta feature that lets the service repeat a declined request
/// on another model (`fallbacks: "default"`, see
/// [`Model::has_fallback`]).
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// The longest reply, in tokens. Long enough for any answer of the
/// chat; a reply that reaches it is cut and marked so.
const MAX_TOKENS: u32 = 64_000;

/// The length of the reply to the check of a key: the reply itself
/// does not matter, only that there is one.
const CHECK_MAX_TOKENS: u32 = 16;

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long the service may stay silent while a reply is written.
/// It sends a "ping" event regularly, so a longer silence means a
/// broken connection.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// How long the whole check of a key may take.
const CHECK_TIMEOUT: Duration = Duration::from_secs(60);

/// What the model is told about its place, before the conversation.
const SYSTEM_PROMPT: &str = "You are the assistant in IO Master, a tool used by Common Criteria \
evaluators in an evaluation laboratory. The person you talk to is an evaluator. Answer precisely \
and say plainly when you are not sure. When you refer to the Common Criteria (ISO/IEC 15408) or \
the CEM (ISO/IEC 18045), name the part and the element, for example ADV_FSP.4.";

/// Who said something in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    /// The evaluator ("user" in the API).
    Evaluator,
    /// The model ("assistant" in the API).
    Model,
}

impl Speaker {
    /// The name of the speaker in the API.
    fn role(self) -> &'static str {
        match self {
            Speaker::Evaluator => "user",
            Speaker::Model => "assistant",
        }
    }
}

/// One message of a conversation, as it is sent to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub speaker: Speaker,
    pub text: String,
}

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
    /// The model that answered, as the service named it; `None`
    /// when the reply did not start.
    pub model: Option<String>,
    /// Tokens of the request (the whole conversation).
    pub input_tokens: u64,
    /// Tokens of the reply.
    pub output_tokens: u64,
    pub end: ReplyEnd,
}

/// The connection to Anthropic. One is made when the program starts
/// and shared: it keeps connections open between requests.
#[derive(Debug, Clone)]
pub struct Anthropic {
    http: Client,
    /// [`SERVICE_URL`]; another address only in the tests.
    service_url: String,
}

/// The settings of the connection that protect the key.
fn protected_client() -> reqwest::ClientBuilder {
    Client::builder()
        .https_only(true)
        .tls_version_min(Version::TLS_1_3)
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .user_agent(concat!("io-master/", env!("CARGO_PKG_VERSION")))
}

impl Anthropic {
    /// The connection to the real service. Fails only when the TLS
    /// library cannot be set up, for example without the trusted
    /// certificates of the system.
    pub fn new() -> Result<Anthropic, AiError> {
        let http = protected_client()
            .build()
            .map_err(|error| AiError::NoConnection(explain(&error)))?;

        Ok(Anthropic {
            http,
            service_url: SERVICE_URL.to_owned(),
        })
    }

    /// A connection to a fake service of the tests, at `url`, whose
    /// certificate is signed by `root` (DER). Everything else is as
    /// for the real service: TLS 1.3, no redirects.
    #[cfg(test)]
    pub fn with_service(url: &str, root: &[u8]) -> Anthropic {
        let root = reqwest::Certificate::from_der(root).expect("the test certificate is valid");

        Anthropic {
            http: protected_client().tls_certs_only([root]).build().expect("the test client is built"),
            service_url: url.to_owned(),
        }
    }

    /// A connection to a service that does not exist (port 9 of
    /// 127.0.0.1, where nothing listens), for the tests that do not
    /// set up a fake service: they never reach the network.
    #[cfg(test)]
    pub fn offline() -> Anthropic {
        Anthropic {
            http: protected_client().build().expect("the test client is built"),
            service_url: "https://127.0.0.1:9".to_owned(),
        }
    }

    /// A request to the Messages API with the key and the headers.
    fn request(&self, key: &ApiKey, model: Model, body: &Value) -> RequestBuilder {
        let mut request = self
            .http
            .post(format!("{}{MESSAGES_PATH}", self.service_url))
            .header("x-api-key", key.expose())
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .body(body.to_string());

        if model.has_fallback() {
            request = request.header("anthropic-beta", FALLBACK_BETA);
        }

        request
    }

    /// Checks that the key is accepted and that the model can be
    /// used with it: one short request, as in v0.2. Also tells
    /// whether the account can pay for requests.
    pub async fn check(&self, key: &ApiKey, model: Model) -> Result<(), AiError> {
        let turns = [Turn {
            speaker: Speaker::Evaluator,
            text: "Reply with the word OK.".to_owned(),
        }];

        let response = self
            .request(key, model, &request_body(model, &turns, CHECK_MAX_TOKENS, false))
            .timeout(CHECK_TIMEOUT)
            .send()
            .await
            .map_err(|error| AiError::NoConnection(explain(&error)))?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status().as_u16();
        let body = response.bytes().await.unwrap_or_default();

        Err(AiError::from_answer(status, &body))
    }

    /// Asks the model for the next reply of a conversation, and
    /// sends the reply into `pieces` while it is written. Stops when
    /// the receiver of `pieces` is dropped (see the top of this
    /// file). Never fails as a whole: how the reply ended is in
    /// [`Reply::end`], next to what was written until then.
    pub async fn reply(&self, key: &ApiKey, model: Model, turns: &[Turn], pieces: &mpsc::Sender<String>) -> Reply {
        let mut reader = ReplyReader::default();

        let request = self
            .request(key, model, &request_body(model, turns, MAX_TOKENS, true))
            .send();

        let sent = tokio::select! {
            sent = request => sent,
            () = pieces.closed() => return finish(reader, ReplyEnd::Cancelled),
        };

        let mut response = match sent {
            Ok(response) => response,
            Err(error) => return finish(reader, ReplyEnd::Failed(AiError::NoConnection(explain(&error)))),
        };

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.bytes().await.unwrap_or_default();

            return finish(reader, ReplyEnd::Failed(AiError::from_answer(status, &body)));
        }

        let mut decoder = SseDecoder::default();

        loop {
            let chunk = tokio::select! {
                chunk = response.chunk() => chunk,
                () = pieces.closed() => return finish(reader, ReplyEnd::Cancelled),
            };

            let bytes = match chunk {
                Ok(Some(bytes)) => bytes,
                Ok(None) => break,
                Err(error) => return finish(reader, ReplyEnd::Failed(AiError::NoConnection(explain(&error)))),
            };

            for event in decoder.push(&bytes) {
                match reader.read(&event) {
                    // A receiver that is gone is noticed by the
                    // select above.
                    Ok(Some(piece)) => {
                        let _ = pieces.send(piece).await;
                    }
                    Ok(None) => {}
                    Err(error) => return finish(reader, ReplyEnd::Failed(error)),
                }
            }
        }

        let end = match (&reader.stop, reader.complete) {
            (Some(stop), true) => ReplyEnd::Finished(stop.clone()),
            (None, true) => ReplyEnd::Finished(StopReason::EndTurn),
            (_, false) => ReplyEnd::Failed(AiError::NoConnection("the reply broke off".to_owned())),
        };

        finish(reader, end)
    }
}

/// The reply from what was read and how it ended.
fn finish(reader: ReplyReader, end: ReplyEnd) -> Reply {
    Reply {
        text: reader.text,
        model: reader.model,
        input_tokens: reader.input_tokens,
        output_tokens: reader.output_tokens,
        end,
    }
}

/// The body of a request. The thinking of the model is left to its
/// defaults (adaptive on the current models) and is not shown.
fn request_body(model: Model, turns: &[Turn], max_tokens: u32, stream: bool) -> Value {
    let mut body = json!({
        "model": model.id(),
        "max_tokens": max_tokens,
        "system": SYSTEM_PROMPT,
        "messages": messages(turns),
    });

    if stream {
        body["stream"] = Value::Bool(true);
    }

    if model.has_fallback() {
        body["fallbacks"] = Value::from("default");
    }

    body
}

/// The messages of the API from the turns of a conversation. The
/// API wants the speakers to take turns and the evaluator to start;
/// a conversation may break this rule when a reply failed (two
/// messages of the evaluator in a row), so such messages are joined
/// into one, and empty messages are left out.
fn messages(turns: &[Turn]) -> Vec<Value> {
    let mut joined: Vec<(Speaker, String)> = Vec::new();

    for turn in turns.iter().filter(|turn| !turn.text.trim().is_empty()) {
        match joined.last_mut() {
            Some((speaker, text)) if *speaker == turn.speaker => {
                text.push_str("\n\n");
                text.push_str(&turn.text);
            }
            _ => joined.push((turn.speaker, turn.text.clone())),
        }
    }

    // A conversation never starts with the model.
    if joined.first().is_some_and(|(speaker, _)| *speaker == Speaker::Model) {
        joined.remove(0);
    }

    joined
        .into_iter()
        .map(|(speaker, text)| json!({"role": speaker.role(), "content": text}))
        .collect()
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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "anthropic_tests.rs"]
mod tests;
