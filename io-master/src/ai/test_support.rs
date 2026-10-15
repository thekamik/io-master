//! A fake Anthropic service for the tests: an HTTPS server on
//! 127.0.0.1 that answers every request with a prepared answer and
//! keeps the requests it received.
//!
//! It speaks real TLS (1.3 by default, or only 1.2 to test that the
//! client refuses it) with a certificate made for the test and
//! signed by a test authority; [`FakeAnthropic::client`] is an
//! [`Anthropic`] that trusts only that authority. No request ever
//! leaves the computer.
//!
//! The HTTP side is the least that works: one request per
//! connection, the answer ends when the connection is closed.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::{self, ServerConfig};

use super::anthropic::Anthropic;

/// What the fake service answers.
#[derive(Debug, Clone)]
pub enum FakeAnswer {
    /// A complete answer: status and JSON body.
    Json(u16, String),
    /// A streamed reply: these events, then the end of the stream.
    Stream(Vec<String>),
    /// A streamed reply that writes these events and then stays
    /// silent until the client gives up.
    StreamThenWait(Vec<String>),
}

/// A request the fake service received.
#[derive(Debug, Clone)]
pub struct FakeRequest {
    /// The first line and the headers, as sent (names in lowercase).
    pub head: String,
    /// The body (JSON).
    pub body: serde_json::Value,
}

impl FakeRequest {
    /// The value of a header, if it was sent.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().find_map(|line| {
            let (field, value) = line.split_once(':')?;

            (field.eq_ignore_ascii_case(name)).then(|| value.trim())
        })
    }
}

/// A running fake service.
pub struct FakeAnthropic {
    /// `https://127.0.0.1:<port>`.
    pub url: String,
    /// The certificate of the test authority (DER).
    root: Vec<u8>,
    /// The requests received so far.
    pub requests: Arc<Mutex<Vec<FakeRequest>>>,
}

/// One event of a streamed reply in the format of the stream.
pub fn sse(name: &str, data: &serde_json::Value) -> String {
    format!("event: {name}\ndata: {data}\n\n")
}

/// The events of a complete streamed reply with the given pieces of
/// text.
pub fn reply_events(pieces: &[&str]) -> Vec<String> {
    use serde_json::json;

    let mut events = vec![
        sse(
            "message_start",
            &json!({"type": "message_start", "message": {"model": "claude-opus-5-5", "usage": {"input_tokens": 10, "output_tokens": 1}}}),
        ),
        sse("content_block_start", &json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})),
    ];

    for piece in pieces {
        events.push(sse(
            "content_block_delta",
            &json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": piece}}),
        ));
    }

    events.push(sse("content_block_stop", &json!({"type": "content_block_stop", "index": 0})));
    events.push(sse(
        "message_delta",
        &json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 5}}),
    ));
    events.push(sse("message_stop", &json!({"type": "message_stop"})));

    events
}

impl FakeAnthropic {
    /// Starts a service that speaks TLS 1.3.
    pub async fn start(answer: FakeAnswer) -> FakeAnthropic {
        FakeAnthropic::start_with(answer, &[&rustls::version::TLS13]).await
    }

    /// Starts a service that speaks only TLS 1.2, which the client
    /// must refuse.
    pub async fn start_tls12(answer: FakeAnswer) -> FakeAnthropic {
        FakeAnthropic::start_with(answer, &[&rustls::version::TLS12]).await
    }

    async fn start_with(answer: FakeAnswer, versions: &[&'static rustls::SupportedProtocolVersion]) -> FakeAnthropic {
        // The test authority, and the certificate of the service
        // for 127.0.0.1 signed by it.
        let authority_key = KeyPair::generate().unwrap();
        let mut authority = CertificateParams::new(Vec::<String>::new()).unwrap();
        authority.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let root = authority.self_signed(&authority_key).unwrap().der().to_vec();
        let issuer = Issuer::new(authority, authority_key);

        let server_key = KeyPair::generate().unwrap();
        let server_certificate = CertificateParams::new(vec!["127.0.0.1".to_owned()])
            .unwrap()
            .signed_by(&server_key, &issuer)
            .unwrap();

        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(versions)
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(server_certificate.der().to_vec())],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
            )
            .unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("https://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&requests);

        tokio::spawn(async move {
            while let Ok((connection, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                let answer = answer.clone();
                let received = Arc::clone(&received);

                tokio::spawn(async move {
                    // A failed handshake (TLS 1.2 refused) ends here.
                    let Ok(mut stream) = acceptor.accept(connection).await else {
                        return;
                    };

                    let Some(request) = read_request(&mut stream).await else {
                        return;
                    };

                    received.lock().unwrap().push(request);

                    let _ = write_answer(&mut stream, &answer).await;
                });
            }
        });

        FakeAnthropic { url, root, requests }
    }

    /// A client that trusts the certificate of this service.
    pub fn client(&self) -> Anthropic {
        Anthropic::with_service(&self.url, &self.root)
    }

    /// The certificate of the test authority of this service (DER).
    pub fn root(&self) -> &[u8] {
        &self.root
    }

    /// The requests received so far.
    pub fn received(&self) -> Vec<FakeRequest> {
        self.requests.lock().unwrap().clone()
    }
}

/// Reads one HTTP/1.1 request: the head up to the empty line, then
/// as many bytes of body as `content-length` says.
async fn read_request<S: AsyncReadExt + Unpin>(stream: &mut S) -> Option<FakeRequest> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];

    let head_end = loop {
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position;
        }

        let count = stream.read(&mut buffer).await.ok()?;

        if count == 0 {
            return None;
        }

        bytes.extend_from_slice(&buffer[..count]);
    };

    let head = lowercase_names(&String::from_utf8_lossy(&bytes[..head_end]));
    let length: usize = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0);

    let mut body = bytes[head_end + 4..].to_vec();

    while body.len() < length {
        let count = stream.read(&mut buffer).await.ok()?;

        if count == 0 {
            return None;
        }

        body.extend_from_slice(&buffer[..count]);
    }

    Some(FakeRequest {
        head,
        body: serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    })
}

/// Writes the prepared answer.
async fn write_answer<S: AsyncWriteExt + Unpin>(stream: &mut S, answer: &FakeAnswer) -> std::io::Result<()> {
    match answer {
        FakeAnswer::Json(status, body) => {
            let head = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );

            stream.write_all(head.as_bytes()).await?;
            stream.write_all(body.as_bytes()).await?;
        }
        FakeAnswer::Stream(events) | FakeAnswer::StreamThenWait(events) => {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n")
                .await?;

            for event in events {
                stream.write_all(event.as_bytes()).await?;
                stream.flush().await?;
            }

            if matches!(answer, FakeAnswer::StreamThenWait(_)) {
                tokio::time::sleep(Duration::from_secs(600)).await;
            }
        }
    }

    stream.flush().await?;
    stream.shutdown().await
}

/// Lowercases the names of the headers (not the first line, not the
/// values), so that the tests can look them up in one spelling.
fn lowercase_names(head: &str) -> String {
    head.lines()
        .enumerate()
        .map(|(index, line)| match line.split_once(':') {
            Some((name, value)) if index > 0 => format!("{}:{value}", name.to_lowercase()),
            _ => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
