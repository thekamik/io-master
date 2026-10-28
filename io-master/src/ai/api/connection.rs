//! A connection to an AI service, as the evaluator sets it up on the
//! tab AI of Settings (step 15 of `docs/history.md`), and the rules it must
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
//! (`ai/api/provider/request.rs`):
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

use std::net::IpAddr;

use reqwest::{Certificate, Url};
use serde_json::Value;

use super::provider::json_path::JsonPath;
use crate::model_old::ai::{
    API_KEY, AnswerMode, Connection, ConnectionError, Field, MAX_NAME, MESSAGES, MODEL, RESERVED_HEADERS, ROLE, SYSTEM,
    TEXT,
};

/// The longest URL, model name, role or end marker, in characters.
const MAX_SHORT: usize = 2000;
/// The longest headers, together, in bytes.
const MAX_HEADERS: usize = 8 * 1024;
/// The longest body template and system prompt, in bytes.
const MAX_TEMPLATE: usize = 64 * 1024;
/// The longest CA certificate (PEM), in bytes.
const MAX_CERTIFICATE: usize = 64 * 1024;

/// A connection error of `field`.
fn refuse<T>(field: Field, reason: impl Into<String>) -> Result<T, ConnectionError> {
    Err(ConnectionError { field, reason: reason.into() })
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
}

/// A connection error, as a value.
fn error(field: Field, reason: &str) -> ConnectionError {
    ConnectionError { field, reason: reason.to_owned() }
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
        Some(text) => JsonPath::parse(&text).map(Some).map_err(|problem| error(field, &format!("{problem}"))),
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

    let body: Value = serde_json::from_str(text)
        .map_err(|problem| error(Field::Body, &format!("the body is not valid JSON: {problem}")))?;

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
                refuse(field, format!("\"{text}\": here a placeholder must be the whole text, one of {named}"))
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

#[cfg(test)]
mod tests {
    //! Unit tests of `connection.rs`.

    use super::*;

    /// The template with a name and a model: a connection that passes.
    fn good() -> Connection {
        Connection { name: "Local".to_owned(), model: "llama3.3:70b".to_owned(), ..Connection::template() }
    }

    /// The field a connection is refused for.
    fn refused_field(connection: &Connection) -> Field {
        connection.check().expect_err("the connection should be refused").field
    }

    #[test]
    fn the_template_with_a_name_and_a_model_passes() {
        let checked = good().check().unwrap();

        assert_eq!(checked.url, "http://127.0.0.1:11434/v1/chat/completions");
        assert_eq!(checked.headers, [("content-type".to_owned(), "application/json".to_owned())]);
        assert_eq!(checked.text_path.as_str(), "choices[0].delta.content");
        assert_eq!(checked.cut_reasons, ["length"]);
        assert_eq!(checked.system_role.as_deref(), Some("system"));
        assert_eq!(checked.done_marker.as_deref(), Some("[DONE]"));
        assert!(!checked.needs_key());
    }

    #[test]
    fn a_name_is_needed() {
        let connection = Connection { name: "  ".to_owned(), ..good() };

        assert_eq!(refused_field(&connection), Field::Name);
    }

    #[test]
    fn https_anywhere_and_http_only_to_the_loopback() {
        for url in
            ["https://ai.lab.example/v1/chat", "http://127.0.0.1:8080/x", "http://127.1.2.3/x", "http://[::1]:8000/x"]
        {
            let connection = Connection { url: url.to_owned(), ..good() };

            assert!(connection.check().is_ok(), "{url} should pass");
        }

        for url in [
            "http://localhost:11434/x",
            "http://192.168.1.10/x",
            "http://ai.lab.example/x",
            "ftp://127.0.0.1/x",
            "not a url",
            "https://user:pass@ai.lab.example/x",
            "https://ai.lab.example/x#part",
        ] {
            let connection = Connection { url: url.to_owned(), ..good() };

            assert_eq!(refused_field(&connection), Field::Url, "{url} should be refused");
        }
    }

    #[test]
    fn the_api_key_is_refused_outside_the_headers() {
        let in_url = Connection { url: "https://ai.lab.example/x?key={{api_key}}".to_owned(), ..good() };
        let in_body = Connection { body: r#"{"messages": "{{messages}}", "key": "{{api_key}}"}"#.to_owned(), ..good() };
        let in_header = Connection { headers: "authorization: Bearer {{api_key}}".to_owned(), ..good() };

        assert_eq!(refused_field(&in_url), Field::Url);
        assert_eq!(refused_field(&in_body), Field::Body);
        assert!(in_header.check().unwrap().needs_key());
    }

    #[test]
    fn the_model_may_be_in_the_url() {
        let connection = Connection { url: "https://ai.lab.example/models/{{model}}:generate".to_owned(), ..good() };

        let checked = connection.check().unwrap();
        assert_eq!(checked.url_with_model(), "https://ai.lab.example/models/llama3.3:70b:generate");
    }

    #[test]
    fn a_used_model_must_have_a_name() {
        let without = Connection { model: String::new(), ..good() };
        assert_eq!(refused_field(&without), Field::Model);

        // A body without {{model}} needs none.
        let unused = Connection { model: String::new(), body: r#"{"messages": "{{messages}}"}"#.to_owned(), ..good() };
        assert!(unused.check().is_ok());

        let spaced = Connection { model: "a model".to_owned(), ..good() };
        assert_eq!(refused_field(&spaced), Field::Model);
    }

    #[test]
    fn placeholders_in_the_body_are_whole_texts() {
        for body in [
            r#"{"model": "{{model}}"}"#,                              // no {{messages}}
            r#"{"messages": "{{messages}}", "p": "say {{system}}"}"#, // inside a text
            r#"{"messages": "{{messages}}", "x": "{{unknown}}"}"#,    // unknown
            r#"{"messages": "{{messages}}", "{{model}}": 1}"#,        // a name of a field
            r#"{"messages": "{{messages}}""#,                         // not JSON
        ] {
            let connection = Connection { body: body.to_owned(), ..good() };

            assert_eq!(refused_field(&connection), Field::Body, "{body} should be refused");
        }

        let nested = Connection {
            body: r#"{"input": {"messages": "{{messages}}", "system": ["{{system}}"]}, "model": "{{model}}"}"#
                .to_owned(),
            ..good()
        };
        assert!(nested.check().is_ok());
    }

    #[test]
    fn headers_are_checked() {
        for headers in [
            "no colon here",
            "bad name: x",
            "host: other.example",
            "content-length: 5",
            "x-thing: {{messages}}",
            "x-thing: zażółć",
        ] {
            let connection = Connection { headers: headers.to_owned(), ..good() };

            assert_eq!(refused_field(&connection), Field::Headers, "{headers} should be refused");
        }

        let connection = Connection {
            headers: "\n  X-Api-Key: {{api_key}}  \n\nanthropic-version: 2023-06-01\n".to_owned(),
            ..good()
        };
        let checked = connection.check().unwrap();

        assert_eq!(
            checked.headers,
            [
                ("x-api-key".to_owned(), "{{api_key}}".to_owned()),
                ("anthropic-version".to_owned(), "2023-06-01".to_owned())
            ]
        );
    }

    #[test]
    fn paths_are_checked_and_the_text_path_is_needed() {
        let without_text = Connection { text_path: String::new(), ..good() };
        assert_eq!(refused_field(&without_text), Field::TextPath);

        let broken_stop = Connection { stop_path: "a..b".to_owned(), ..good() };
        assert_eq!(refused_field(&broken_stop), Field::StopPath);

        let optional_empty =
            Connection { stop_path: String::new(), error_path: " ".to_owned(), done_marker: String::new(), ..good() };
        let checked = optional_empty.check().unwrap();
        assert!(checked.stop_path.is_none() && checked.error_path.is_none() && checked.done_marker.is_none());
    }

    #[test]
    fn reasons_are_a_list() {
        let connection =
            Connection { cut_reasons: " max_tokens, ,model_context_window_exceeded ".to_owned(), ..good() };

        assert_eq!(connection.check().unwrap().cut_reasons, ["max_tokens", "model_context_window_exceeded"]);
    }

    #[test]
    fn a_ca_certificate_must_be_pem() {
        let broken = Connection { ca_certificate: "not a certificate".to_owned(), ..good() };
        assert_eq!(refused_field(&broken), Field::CaCertificate);

        let key = rcgen::KeyPair::generate().unwrap();
        let pem =
            rcgen::CertificateParams::new(vec!["ai.lab.example".to_owned()]).unwrap().self_signed(&key).unwrap().pem();
        let with_ca = Connection { ca_certificate: pem, ..good() };

        assert_eq!(with_ca.check().unwrap().ca_certificates.map(|list| list.len()), Some(1));
    }

    #[test]
    fn the_origin_is_scheme_host_and_port() {
        let connection = Connection { url: "https://ai.lab.example:8443/v1/{{model}}/chat".to_owned(), ..good() };

        assert_eq!(connection.origin().as_deref(), Some("https://ai.lab.example:8443"));
        assert_eq!(Connection { url: "nonsense".to_owned(), ..good() }.origin(), None);
    }

    #[test]
    fn modes_have_names() {
        for mode in [AnswerMode::Stream, AnswerMode::Whole] {
            assert_eq!(AnswerMode::from_name(mode.name()), Some(mode));
        }

        assert_eq!(AnswerMode::from_name("other"), None);
    }

    #[test]
    fn the_message_template_is_checked() {
        for template in [
            r#"{"role": "{{role}}"}"#,                            // no {{text}}
            r#"{"role": "{{role}}", "content": "say {{text}}"}"#, // inside a text
            r#"{"content": "{{text}}", "m": "{{model}}"}"#,       // not a placeholder of a message
            r#"{"content": "{{text}}""#,                          // not JSON
        ] {
            let connection = Connection { message_template: template.to_owned(), ..good() };

            assert_eq!(refused_field(&connection), Field::MessageTemplate, "{template} should be refused");
        }

        let parts = Connection {
            message_template: r#"{"role": "{{role}}", "parts": [{"text": "{{text}}"}]}"#.to_owned(),
            ..good()
        };
        assert!(parts.check().is_ok());
    }

    #[test]
    fn every_field_is_found_by_its_name() {
        for field in Field::ALL {
            assert_eq!(Field::from_name(field.name()), Some(field));
        }

        assert_eq!(Field::from_name("api_key"), None);
    }
}
