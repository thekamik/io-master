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
use crate::model::ai::AnswerMode;
use crate::model::ai::{Speaker, Turn};
use crate::model::api_key::ApiKey;

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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
