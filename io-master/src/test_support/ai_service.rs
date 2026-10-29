//! A fake AI service for the tests: a server on 127.0.0.1 that answers
//! every request with a prepared answer and keeps the requests it got.
//!
//! It speaks real TLS (1.3 by default, or only 1.2 to test that the client
//! refuses it) with a certificate made for the test and signed by an
//! authority of the test, or plain HTTP (which the program allows only on
//! this computer). No request leaves the computer. One request per
//! connection; the answer ends when the connection is closed.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::{self, ServerConfig};

/// What the fake service answers.
#[derive(Debug, Clone)]
pub enum FakeAnswer {
    /// A whole answer: its status and its body.
    Json(u16, String),
    /// A streamed answer: these events, then the end of the stream.
    Stream(Vec<String>),
    /// A streamed answer that writes these events and then stays silent
    /// until the client gives up.
    StreamThenWait(Vec<String>),
}

/// A request the fake service got.
#[derive(Debug, Clone)]
pub struct FakeRequest {
    /// The first line and the headers, as sent (names in lowercase).
    pub head: String,
    /// The body (JSON).
    pub body: Value,
}

impl FakeRequest {
    /// The value of a header, if it was sent.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().find_map(|line| {
            let (field, value) = line.split_once(':')?;

            field.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }
}

/// A running fake service.
pub struct FakeService {
    /// `https://127.0.0.1:<port>` (or `http://` without TLS).
    pub url: String,
    /// The certificate of the authority of the test (PEM); `None` without
    /// TLS.
    pub ca_pem: Option<String>,
    requests: Arc<Mutex<Vec<FakeRequest>>>,
}

/// One event of a stream; without a name when `name` is empty.
pub fn sse(name: &str, data: &Value) -> String {
    if name.is_empty() { format!("data: {data}\n\n") } else { format!("event: {name}\ndata: {data}\n\n") }
}

/// The events of a whole answer in the protocol of OpenAI, with these
/// pieces of text.
pub fn openai_events(pieces: &[&str]) -> Vec<String> {
    let mut events: Vec<String> = pieces
        .iter()
        .map(|piece| sse("", &json!({"model": "local-model", "choices": [{"delta": {"content": piece}}]})))
        .collect();

    events.push(sse("", &json!({"choices": [{"delta": {}, "finish_reason": "stop"}]})));
    events.push(sse("", &json!({"choices": [], "usage": {"prompt_tokens": 10, "completion_tokens": 5}})));
    events.push("data: [DONE]\n\n".to_owned());
    events
}

/// The events of a whole answer in the protocol of Anthropic, with these
/// pieces of text.
pub fn anthropic_events(pieces: &[&str]) -> Vec<String> {
    let mut events = vec![sse(
        "message_start",
        &json!({"type": "message_start", "message": {"model": "named-model", "usage": {"input_tokens": 10}}}),
    )];

    for piece in pieces {
        events.push(sse(
            "content_block_delta",
            &json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": piece}}),
        ));
    }

    events.push(sse(
        "message_delta",
        &json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 5}}),
    ));
    events.push(sse("message_stop", &json!({"type": "message_stop"})));
    events
}

impl FakeService {
    /// Starts a service that speaks TLS 1.3.
    pub async fn start(answer: FakeAnswer) -> FakeService {
        FakeService::start_tls(answer, &[&rustls::version::TLS13]).await
    }

    /// Starts a service that speaks only TLS 1.2, which the client refuses.
    pub async fn start_tls12(answer: FakeAnswer) -> FakeService {
        FakeService::start_tls(answer, &[&rustls::version::TLS12]).await
    }

    /// Starts a service with plain HTTP on 127.0.0.1.
    pub async fn start_plain(answer: FakeAnswer) -> FakeService {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&requests);

        tokio::spawn(async move {
            while let Ok((connection, _)) = listener.accept().await {
                tokio::spawn(serve(connection, answer.clone(), Arc::clone(&received)));
            }
        });

        FakeService { url, ca_pem: None, requests }
    }

    async fn start_tls(answer: FakeAnswer, versions: &[&'static rustls::SupportedProtocolVersion]) -> FakeService {
        // The authority of the test, and the certificate of the service for
        // 127.0.0.1 signed by it.
        let authority_key = KeyPair::generate().unwrap();
        let mut authority = CertificateParams::new(Vec::<String>::new()).unwrap();

        authority.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

        let ca_pem = authority.self_signed(&authority_key).unwrap().pem();
        let issuer = Issuer::new(authority, authority_key);
        let server_key = KeyPair::generate().unwrap();
        let server_certificate =
            CertificateParams::new(vec!["127.0.0.1".to_owned()]).unwrap().signed_by(&server_key, &issuer).unwrap();
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
                    if let Ok(stream) = acceptor.accept(connection).await {
                        serve(stream, answer, received).await;
                    }
                });
            }
        });

        FakeService { url, ca_pem: Some(ca_pem), requests }
    }

    /// The requests got so far.
    pub fn received(&self) -> Vec<FakeRequest> {
        self.requests.lock().unwrap().clone()
    }
}

/// Reads one request, keeps it and writes the prepared answer.
async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    answer: FakeAnswer,
    received: Arc<Mutex<Vec<FakeRequest>>>,
) {
    let Some(request) = read_request(&mut stream).await else {
        return;
    };

    received.lock().unwrap().push(request);

    let _ = write_answer(&mut stream, &answer).await;
}

/// Reads one HTTP/1.1 request: the head up to the empty line, then as many
/// bytes of body as `content-length` says.
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

    Some(FakeRequest { head, body: serde_json::from_slice(&body).unwrap_or(Value::Null) })
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
/// values), so that the tests look them up in one spelling.
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
