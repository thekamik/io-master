//! A connection to an AI service, as the evaluator sets it up on the
//! tab AI of Settings (step 15 of `docs/todo.md`), and the rules it must
//! keep.
//!
//! The program knows no provider. Everything about the service is in
//! the connection: where it is, how a request looks, and where in
//! the answer the text is:
//!
//! ```text
//! url           https://ai.lab.example/v1/chat/completions
//! headers       authorization: Bearer {{api_key}}
//! body          {"model": "{{model}}", "messages": "{{messages}}", "stream": true}
//! answer        a stream of events, or one JSON
//! text path     choices[0].delta.content
//! ```
//!
//! [`Connection`] holds the texts as they were entered (and as they
//! are stored); [`Connection::check`] checks them and gives the
//! [`Checked`] connection that a request is made from. A connection
//! that breaks a rule is never used and never stored.
//!
//! **Placeholders** are replaced when a request is made
//! (`ai/request.rs`):
//!
//! ```text
//! {{messages}}  the conversation: a list with one message template
//!               filled for every message
//! {{system}}    the instructions of the program for the model
//! {{model}}     the name of the model
//! {{api_key}}   the API key
//! ```
//!
//! The **message template** says how one message looks, for example
//! `{"role": "{{role}}", "content": "{{text}}"}` for most services,
//! or `{"role": "{{role}}", "parts": [{"text": "{{text}}"}]}` for
//! another; `{{role}}` is the role of the speaker (the roles of the
//! connection), `{{text}}` the text of the message.
//!
//! **The rules that protect the key and the conversation:**
//!
//! - `https://` only, or `http://` to the loopback (`127.0.0.0/8`,
//!   `::1`), whose traffic does not leave the computer. The loopback
//!   is written as an IP address: a name such as `localhost` is
//!   resolved by the system and could lead elsewhere.
//! - `{{api_key}}` only in a header: never in the URL or the body,
//!   which servers often write into their logs.
//! - In the body a placeholder is a whole JSON text (`"{{model}}"`),
//!   replaced by a value, never by joining text, so a message can
//!   never break the JSON or add a field to the request.

use std::fmt;
use std::net::IpAddr;

use reqwest::{Certificate, Url};
use serde_json::Value;

use super::json_path::JsonPath;

/// The placeholder of the conversation.
pub const MESSAGES: &str = "{{messages}}";
/// The placeholder of the instructions of the program.
pub const SYSTEM: &str = "{{system}}";
/// The placeholder of the name of the model.
pub const MODEL: &str = "{{model}}";
/// The placeholder of the API key.
pub const API_KEY: &str = "{{api_key}}";
/// The placeholder of the role of a speaker, in the message template.
pub const ROLE: &str = "{{role}}";
/// The placeholder of the text of a message, in the message template.
pub const TEXT: &str = "{{text}}";

/// The message template of a new connection.
pub const DEFAULT_MESSAGE_TEMPLATE: &str = "{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}";

/// The longest name of a connection, in characters.
pub const MAX_NAME: usize = 60;
/// The longest URL, model name, role or end marker, in characters.
const MAX_SHORT: usize = 2000;
/// The longest headers, together, in bytes.
const MAX_HEADERS: usize = 8 * 1024;
/// The longest body template and system prompt, in bytes.
const MAX_TEMPLATE: usize = 64 * 1024;
/// The longest CA certificate (PEM), in bytes.
const MAX_CERTIFICATE: usize = 64 * 1024;

/// Headers the program sets itself, or that would change how the
/// request travels; a connection cannot set them.
pub const RESERVED_HEADERS: [&str; 6] = ["host", "content-length", "transfer-encoding", "connection", "upgrade", "te"];

/// The instructions of the program, the default of a new connection.
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are the assistant in IO Master, a tool used by Common \
Criteria evaluators in an evaluation laboratory. The person you talk to is an evaluator. Answer \
precisely and say plainly when you are not sure. When you refer to the Common Criteria (ISO/IEC \
15408) or the CEM (ISO/IEC 18045), name the part and the element, for example ADV_FSP.4.";

/// How the service sends its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerMode {
    /// A stream of server-sent events while the reply is written
    /// (`ai/sse.rs`); every event is JSON, read with the paths.
    Stream,
    /// One JSON when the reply is complete, read with the paths.
    Whole,
}

impl AnswerMode {
    /// The name kept in the database and used by the page.
    pub fn name(self) -> &'static str {
        match self {
            AnswerMode::Stream => "stream",
            AnswerMode::Whole => "whole",
        }
    }

    /// The mode from its name.
    pub fn from_name(name: &str) -> Option<AnswerMode> {
        match name {
            "stream" => Some(AnswerMode::Stream),
            "whole" => Some(AnswerMode::Whole),
            _ => None,
        }
    }
}

/// A connection as it was entered. Every text is kept as written;
/// an empty text means "not used" where a field is optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// The name the evaluator gave it: "Ollama on this computer".
    pub name: String,
    /// The full address of the endpoint; may hold `{{model}}`.
    pub url: String,
    /// The name of the model, for `{{model}}`; may be empty when no
    /// template uses it.
    pub model: String,
    /// The CA certificate (PEM) that alone is trusted for this
    /// service; empty: the trusted certificates of the system.
    pub ca_certificate: String,
    /// The headers, one per line: `name: value`.
    pub headers: String,
    /// The JSON template of the body.
    pub body: String,
    /// The JSON template of one message in `{{messages}}`.
    pub message_template: String,
    /// The role of the evaluator in `{{messages}}`: "user".
    pub user_role: String,
    /// The role of the model in `{{messages}}`: "assistant".
    pub model_role: String,
    /// The role under which the instructions are put first into
    /// `{{messages}}` ("system"); empty: they are not put there
    /// (a service that takes them in a field of its own uses
    /// `{{system}}`).
    pub system_role: String,
    /// The instructions of the program for the model.
    pub system_prompt: String,
    /// How the answer comes.
    pub answer: AnswerMode,
    /// Where the text is: in every event of a stream, or in the
    /// whole answer.
    pub text_path: String,
    /// Where the reason of the end of the reply is (optional).
    pub stop_path: String,
    /// The reasons that mean the reply was cut (comma-separated).
    pub cut_reasons: String,
    /// The reasons that mean the model declined (comma-separated).
    pub refused_reasons: String,
    /// Where the explanation of an error is (optional).
    pub error_path: String,
    /// Where the name of the model that answered is (optional).
    pub model_path: String,
    /// Where the count of tokens of the request is (optional).
    pub input_tokens_path: String,
    /// Where the count of tokens of the reply is (optional).
    pub output_tokens_path: String,
    /// The data of the event that ends a stream (optional): "[DONE]".
    pub done_marker: String,
}

impl Connection {
    /// The starting point of a new connection: the request and the
    /// answer of the chat API that most local servers offer
    /// (`/v1/chat/completions`, streamed). Every field can be
    /// changed; nothing in the program depends on these values.
    pub fn template() -> Connection {
        Connection {
            name: String::new(),
            url: "http://127.0.0.1:11434/v1/chat/completions".to_owned(),
            model: String::new(),
            ca_certificate: String::new(),
            headers: "content-type: application/json".to_owned(),
            body: "{\n  \"model\": \"{{model}}\",\n  \"messages\": \"{{messages}}\",\n  \"stream\": true\n}".to_owned(),
            message_template: DEFAULT_MESSAGE_TEMPLATE.to_owned(),
            user_role: "user".to_owned(),
            model_role: "assistant".to_owned(),
            system_role: "system".to_owned(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_owned(),
            answer: AnswerMode::Stream,
            text_path: "choices[0].delta.content".to_owned(),
            stop_path: "choices[0].finish_reason".to_owned(),
            cut_reasons: "length".to_owned(),
            refused_reasons: "content_filter".to_owned(),
            error_path: "error.message".to_owned(),
            model_path: "model".to_owned(),
            input_tokens_path: "usage.prompt_tokens".to_owned(),
            output_tokens_path: "usage.completion_tokens".to_owned(),
            done_marker: "[DONE]".to_owned(),
        }
    }
}

/// A field of a connection: to say which one breaks a rule, and to
/// explain one (`ai/field_help.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Url,
    Model,
    CaCertificate,
    Headers,
    Body,
    MessageTemplate,
    UserRole,
    ModelRole,
    SystemRole,
    SystemPrompt,
    Answer,
    TextPath,
    StopPath,
    CutReasons,
    RefusedReasons,
    ErrorPath,
    ModelPath,
    InputTokensPath,
    OutputTokensPath,
    DoneMarker,
}

impl Field {
    /// Every field, in the order of the form.
    pub const ALL: [Field; 21] = [
        Field::Name,
        Field::Url,
        Field::Model,
        Field::CaCertificate,
        Field::Headers,
        Field::Body,
        Field::MessageTemplate,
        Field::UserRole,
        Field::ModelRole,
        Field::SystemRole,
        Field::SystemPrompt,
        Field::Answer,
        Field::TextPath,
        Field::StopPath,
        Field::CutReasons,
        Field::RefusedReasons,
        Field::ErrorPath,
        Field::ModelPath,
        Field::InputTokensPath,
        Field::OutputTokensPath,
        Field::DoneMarker,
    ];

    /// The field from its name; `None` for a name no field has.
    pub fn from_name(name: &str) -> Option<Field> {
        Field::ALL.into_iter().find(|field| field.name() == name)
    }

    /// The name of the field, as the page knows it.
    pub fn name(self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Url => "url",
            Field::Model => "model",
            Field::CaCertificate => "ca_certificate",
            Field::Headers => "headers",
            Field::Body => "body",
            Field::MessageTemplate => "message_template",
            Field::UserRole => "user_role",
            Field::ModelRole => "model_role",
            Field::SystemRole => "system_role",
            Field::SystemPrompt => "system_prompt",
            Field::Answer => "answer",
            Field::TextPath => "text_path",
            Field::StopPath => "stop_path",
            Field::CutReasons => "cut_reasons",
            Field::RefusedReasons => "refused_reasons",
            Field::ErrorPath => "error_path",
            Field::ModelPath => "model_path",
            Field::InputTokensPath => "input_tokens_path",
            Field::OutputTokensPath => "output_tokens_path",
            Field::DoneMarker => "done_marker",
        }
    }
}

/// Why a connection cannot be used: the field, and what is wrong
/// with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionError {
    pub field: Field,
    pub reason: String,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl std::error::Error for ConnectionError {}

/// A connection error of `field`.
fn refuse<T>(field: Field, reason: impl Into<String>) -> Result<T, ConnectionError> {
    Err(ConnectionError {
        field,
        reason: reason.into(),
    })
}

/// A checked connection: everything a request is made from.
#[derive(Debug, Clone)]
pub struct Checked {
    /// The URL; `{{model}}` in it is replaced when the request is
    /// made.
    pub url: String,
    pub model: String,
    /// The certificates that alone are trusted; `None`: those of
    /// the system.
    pub ca_certificates: Option<Vec<Certificate>>,
    /// The headers: name (lowercase) and value, which may hold
    /// `{{api_key}}` and `{{model}}`.
    pub headers: Vec<(String, String)>,
    /// The template of the body, with its placeholders.
    pub body: Value,
    /// The template of one message, with `{{role}}` and `{{text}}`.
    pub message_template: Value,
    pub user_role: String,
    pub model_role: String,
    pub system_role: Option<String>,
    pub system_prompt: String,
    pub answer: AnswerMode,
    pub text_path: JsonPath,
    pub stop_path: Option<JsonPath>,
    pub cut_reasons: Vec<String>,
    pub refused_reasons: Vec<String>,
    pub error_path: Option<JsonPath>,
    pub model_path: Option<JsonPath>,
    pub input_tokens_path: Option<JsonPath>,
    pub output_tokens_path: Option<JsonPath>,
    pub done_marker: Option<String>,
}

impl Checked {
    /// Whether a header uses the API key: then a request needs one.
    pub fn needs_key(&self) -> bool {
        self.headers.iter().any(|(_, value)| value.contains(API_KEY))
    }

    /// The URL with the name of the model in it.
    pub fn url_with_model(&self) -> String {
        self.url.replace(MODEL, &self.model)
    }
}

impl Connection {
    /// Checks every field against the rules at the top of this file.
    pub fn check(&self) -> Result<Checked, ConnectionError> {
        check_name(&self.name)?;

        let model = check_model(&self.model)?;
        let url = check_url(&self.url, &model)?;
        let ca_certificates = check_certificate(&self.ca_certificate)?;
        let headers = check_headers(&self.headers)?;
        let body = check_body(&self.body)?;
        let message_template = check_message_template(&self.message_template)?;

        // A template that names the model needs one.
        let model_used = self.url.contains(MODEL)
            || headers.iter().any(|(_, value)| value.contains(MODEL))
            || mentions(&body, MODEL);

        if model_used && model.is_empty() {
            return refuse(Field::Model, "the request uses {{model}}, so the name of the model is needed");
        }

        let system_role = optional(&self.system_role);

        if let Some(role) = &system_role {
            check_short(Field::SystemRole, role, "the role of the instructions")?;
        }

        if self.system_prompt.len() > MAX_TEMPLATE {
            return refuse(Field::SystemPrompt, "the instructions are longer than 64 KiB");
        }

        Ok(Checked {
            url,
            model,
            ca_certificates,
            headers,
            body,
            message_template,
            user_role: check_role(Field::UserRole, &self.user_role, "the role of the evaluator")?,
            model_role: check_role(Field::ModelRole, &self.model_role, "the role of the model")?,
            system_role,
            system_prompt: self.system_prompt.clone(),
            answer: self.answer,
            text_path: path(Field::TextPath, &self.text_path)?
                .ok_or_else(|| error(Field::TextPath, "the path to the text is needed"))?,
            stop_path: path(Field::StopPath, &self.stop_path)?,
            cut_reasons: list(&self.cut_reasons),
            refused_reasons: list(&self.refused_reasons),
            error_path: path(Field::ErrorPath, &self.error_path)?,
            model_path: path(Field::ModelPath, &self.model_path)?,
            input_tokens_path: path(Field::InputTokensPath, &self.input_tokens_path)?,
            output_tokens_path: path(Field::OutputTokensPath, &self.output_tokens_path)?,
            done_marker: match optional(&self.done_marker) {
                Some(marker) => Some(check_short(Field::DoneMarker, &marker, "the end marker")?),
                None => None,
            },
        })
    }

    /// The scheme, host and port of the URL: the address the API
    /// key is bound to (`ai/connections.rs`). `None` when the URL is
    /// not valid.
    pub fn origin(&self) -> Option<String> {
        let url = Url::parse(&self.url.trim().replace(MODEL, "model")).ok()?;

        Some(url.origin().ascii_serialization())
    }
}

/// A connection error, as a value.
fn error(field: Field, reason: &str) -> ConnectionError {
    ConnectionError {
        field,
        reason: reason.to_owned(),
    }
}

/// The text without spaces around it; `None` when nothing is left.
fn optional(text: &str) -> Option<String> {
    let text = text.trim();

    (!text.is_empty()).then(|| text.to_owned())
}

/// The comma-separated values of a list, without spaces and empty
/// values.
fn list(text: &str) -> Vec<String> {
    text.split(',').map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned).collect()
}

/// An optional path.
fn path(field: Field, text: &str) -> Result<Option<JsonPath>, ConnectionError> {
    match optional(text) {
        Some(text) => JsonPath::parse(&text)
            .map(Some)
            .map_err(|problem| error(field, &format!("{problem}"))),
        None => Ok(None),
    }
}

/// A short text of visible characters (a role, a marker).
fn check_short(field: Field, text: &str, what: &str) -> Result<String, ConnectionError> {
    if text.chars().count() > MAX_SHORT {
        return refuse(field, format!("{what} is too long"));
    }

    if text.chars().any(char::is_control) {
        return refuse(field, format!("{what} holds a control character"));
    }

    Ok(text.to_owned())
}

/// A role in `{{messages}}`: needed.
fn check_role(field: Field, text: &str, what: &str) -> Result<String, ConnectionError> {
    match optional(text) {
        Some(role) => check_short(field, &role, what),
        None => refuse(field, format!("{what} is needed")),
    }
}

fn check_name(name: &str) -> Result<(), ConnectionError> {
    let name = name.trim();

    if name.is_empty() {
        return refuse(Field::Name, "the connection needs a name");
    }

    if name.chars().count() > MAX_NAME {
        return refuse(Field::Name, format!("the name is longer than {MAX_NAME} characters"));
    }

    if name.chars().any(char::is_control) {
        return refuse(Field::Name, "the name holds a control character");
    }

    Ok(())
}

/// The model name: visible ASCII only, because it may go into a
/// header or the URL.
fn check_model(model: &str) -> Result<String, ConnectionError> {
    let model = model.trim();

    if model.len() > MAX_SHORT {
        return refuse(Field::Model, "the name of the model is too long");
    }

    if !model.bytes().all(|byte| byte.is_ascii_graphic()) {
        return refuse(Field::Model, "the name of the model holds a space or a character outside ASCII");
    }

    Ok(model.to_owned())
}

/// The URL: the rules of the network at the top of this file.
fn check_url(text: &str, model: &str) -> Result<String, ConnectionError> {
    let text = text.trim();

    if text.len() > MAX_SHORT {
        return refuse(Field::Url, "the URL is too long");
    }

    if text.contains(API_KEY) {
        return refuse(Field::Url, "the API key may be used only in a header, never in the URL");
    }

    // Any placeholder other than {{model}} has no meaning here.
    if text.replace(MODEL, "").contains("{{") {
        return refuse(Field::Url, "the URL may use only the placeholder {{model}}");
    }

    let url = Url::parse(&text.replace(MODEL, if model.is_empty() { "model" } else { model }))
        .map_err(|problem| error(Field::Url, &format!("the URL is not valid: {problem}")))?;

    if !url.username().is_empty() || url.password().is_some() {
        return refuse(Field::Url, "the URL may not hold a name or a password; use a header");
    }

    if url.fragment().is_some() {
        return refuse(Field::Url, "the URL may not hold a fragment (#...)");
    }

    match url.scheme() {
        "https" => {}
        "http" => {
            // Only an IP address counts; an IPv6 address is written
            // in brackets ("[::1]"). A name never parses as one.
            let host = url.host_str().unwrap_or("");
            let loopback = host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback());

            if !loopback {
                return refuse(
                    Field::Url,
                    "plain http:// is allowed only to this computer, written as 127.0.0.1 or [::1]; \
                     any other address needs https://",
                );
            }
        }
        _ => return refuse(Field::Url, "the URL must start with https:// (or http:// to 127.0.0.1)"),
    }

    Ok(text.to_owned())
}

/// The CA certificate: one or more certificates in PEM.
fn check_certificate(text: &str) -> Result<Option<Vec<Certificate>>, ConnectionError> {
    let text = text.trim();

    if text.is_empty() {
        return Ok(None);
    }

    if text.len() > MAX_CERTIFICATE {
        return refuse(Field::CaCertificate, "the certificate is longer than 64 KiB");
    }

    let certificates = Certificate::from_pem_bundle(text.as_bytes())
        .map_err(|_| error(Field::CaCertificate, "the text is not a certificate in PEM"))?;

    if certificates.is_empty() {
        return refuse(Field::CaCertificate, "no certificate (-----BEGIN CERTIFICATE-----) was found");
    }

    Ok(Some(certificates))
}

/// Whether a byte may be part of the name of a header (a "token" of
/// HTTP).
fn token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

/// The headers: `name: value`, one per line; empty lines are passed
/// over.
fn check_headers(text: &str) -> Result<Vec<(String, String)>, ConnectionError> {
    if text.len() > MAX_HEADERS {
        return refuse(Field::Headers, "the headers are longer than 8 KiB");
    }

    let mut headers = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            return refuse(Field::Headers, format!("the header line \"{line}\" has no \"name: value\""));
        };

        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();

        if name.is_empty() || !name.bytes().all(token_byte) {
            return refuse(Field::Headers, format!("\"{name}\" is not a valid name of a header"));
        }

        if RESERVED_HEADERS.contains(&name.as_str()) {
            return refuse(Field::Headers, format!("the header {name} is set by the program itself"));
        }

        // Visible ASCII and spaces: what a header may carry. The
        // line breaks were split off above.
        if !value.bytes().all(|byte| byte.is_ascii_graphic() || byte == b' ' || byte == b'\t') {
            return refuse(Field::Headers, format!("the header {name} holds a character outside ASCII"));
        }

        if value.replace(API_KEY, "").replace(MODEL, "").contains("{{") {
            return refuse(Field::Headers, "a header may use only the placeholders {{api_key}} and {{model}}");
        }

        headers.push((name, value.to_owned()));
    }

    Ok(headers)
}

/// The body: JSON, with placeholders only as whole texts.
fn check_body(text: &str) -> Result<Value, ConnectionError> {
    if text.len() > MAX_TEMPLATE {
        return refuse(Field::Body, "the body is longer than 64 KiB");
    }

    let body: Value =
        serde_json::from_str(text).map_err(|problem| error(Field::Body, &format!("the body is not valid JSON: {problem}")))?;

    check_placeholders(&body)?;

    if !mentions(&body, MESSAGES) {
        return refuse(Field::Body, "the body must hold \"{{messages}}\", where the conversation goes");
    }

    Ok(body)
}

/// Every text with `{{` in the body must be exactly one placeholder
/// of the body; names of fields may hold none.
fn check_placeholders(value: &Value) -> Result<(), ConnectionError> {
    check_whole(value, Field::Body, &[MESSAGES, SYSTEM, MODEL], "\"{{messages}}\", \"{{system}}\" or \"{{model}}\"")
}

/// The message template: JSON with `"{{text}}"` (and usually
/// `"{{role}}"`), each a whole text, as in the body.
fn check_message_template(text: &str) -> Result<Value, ConnectionError> {
    if text.len() > MAX_TEMPLATE {
        return refuse(Field::MessageTemplate, "the message template is longer than 64 KiB");
    }

    let template: Value = serde_json::from_str(text).map_err(|problem| {
        error(Field::MessageTemplate, &format!("the message template is not valid JSON: {problem}"))
    })?;

    check_whole(&template, Field::MessageTemplate, &[ROLE, TEXT], "\"{{role}}\" or \"{{text}}\"")?;

    if !mentions(&template, TEXT) {
        return refuse(Field::MessageTemplate, "the message template must hold \"{{text}}\", where the text goes");
    }

    Ok(template)
}

/// Every text with `{{` in `value` must be exactly one of `allowed`;
/// names of fields may hold none. `named` lists them for the message.
fn check_whole(value: &Value, field: Field, allowed: &[&str], named: &str) -> Result<(), ConnectionError> {
    match value {
        Value::String(text) if text.contains("{{") => {
            if allowed.contains(&text.as_str()) {
                Ok(())
            } else if text.contains(API_KEY) {
                refuse(field, "the API key may be used only in a header, never in the body")
            } else {
                refuse(
                    field,
                    format!("\"{text}\": here a placeholder must be the whole text, one of {named}"),
                )
            }
        }
        Value::Array(items) => items.iter().try_for_each(|item| check_whole(item, field, allowed, named)),
        Value::Object(fields) => fields.iter().try_for_each(|(name, value)| {
            if name.contains("{{") {
                return refuse(field, "a name of a field cannot be a placeholder");
            }

            check_whole(value, field, allowed, named)
        }),
        _ => Ok(()),
    }
}

/// Whether the body holds the placeholder somewhere.
fn mentions(value: &Value, placeholder: &str) -> bool {
    match value {
        Value::String(text) => text == placeholder,
        Value::Array(items) => items.iter().any(|item| mentions(item, placeholder)),
        Value::Object(fields) => fields.values().any(|item| mentions(item, placeholder)),
        _ => false,
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
