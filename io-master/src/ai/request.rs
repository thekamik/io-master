//! The request to the AI service, made from the template of a
//! connection (`ai/connection.rs`) and the conversation.
//!
//! The placeholders are replaced here:
//!
//! - in the body, as **values**: `"{{messages}}"` becomes a list with
//!   the message template filled for every message (`"{{role}}"` and
//!   `"{{text}}"` become texts), `"{{system}}"` and `"{{model}}"`
//!   become texts. The body
//!   is a tree of JSON values, and a value is put in place of a
//!   value; the text of a message is never joined into JSON text, so
//!   whatever the evaluator writes (quotes, braces) stays text;
//! - in the URL and the headers, as text: `{{model}}` (checked to be
//!   visible ASCII) and, in headers only, `{{api_key}}` (likewise).
//!
//! [`preview`] makes the same request with the key masked, to show
//! it to the evaluator in the report of a check.

use serde_json::Value;
use zeroize::Zeroizing;

use super::ai_error::AiError;
use super::api_key::ApiKey;
use super::connection::{API_KEY, Checked, MESSAGES, MODEL, ROLE, SYSTEM, TEXT};

/// What stands in place of the key where a request is shown.
pub const MASK: &str = "••••••••";

/// Who said something in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    /// The evaluator (the "user role" of the connection).
    Evaluator,
    /// The model (the "model role" of the connection).
    Model,
}

/// One message of a conversation, as it is sent to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub speaker: Speaker,
    pub text: String,
}

/// A request, ready to be sent.
#[derive(Debug)]
pub struct Request {
    pub url: String,
    /// Name and value. A value may hold the API key, so it is wiped
    /// when the request is dropped.
    pub headers: Vec<(String, Zeroizing<String>)>,
    pub body: Value,
}

/// The request for the conversation `turns`. Fails when a header
/// uses the API key and none is stored.
pub fn request(connection: &Checked, turns: &[Turn], key: Option<&ApiKey>) -> Result<Request, AiError> {
    let key = match (connection.needs_key(), key) {
        (true, None) => return Err(AiError::NoKey),
        (_, key) => key.map(ApiKey::expose).unwrap_or(""),
    };

    Ok(build(connection, turns, key))
}

/// The same request with [`MASK`] in place of the key, to be shown.
pub fn preview(connection: &Checked, turns: &[Turn]) -> Request {
    build(connection, turns, MASK)
}

fn build(connection: &Checked, turns: &[Turn], key: &str) -> Request {
    let headers = connection
        .headers
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                Zeroizing::new(value.replace(API_KEY, key).replace(MODEL, &connection.model)),
            )
        })
        .collect();

    Request {
        url: connection.url_with_model(),
        headers,
        body: fill(&connection.body, connection, &messages(connection, turns)),
    }
}

/// The body template with its placeholders replaced by values.
fn fill(template: &Value, connection: &Checked, messages: &Value) -> Value {
    match template {
        Value::String(text) => match text.as_str() {
            MESSAGES => messages.clone(),
            SYSTEM => Value::String(connection.system_prompt.clone()),
            MODEL => Value::String(connection.model.clone()),
            _ => template.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(|item| fill(item, connection, messages)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), fill(value, connection, messages)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The value of `{{messages}}`: the instructions first (when the
/// connection gives them a role), then the conversation.
///
/// Most services want the speakers to take turns and the evaluator
/// to start; a conversation may break this when a reply failed (two
/// messages of the evaluator in a row), so such messages are joined
/// into one, empty messages are left out, and a reply of the model
/// at the start is dropped.
fn messages(connection: &Checked, turns: &[Turn]) -> Value {
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

    if joined.first().is_some_and(|(speaker, _)| *speaker == Speaker::Model) {
        joined.remove(0);
    }

    let mut list = Vec::new();

    if let Some(role) = &connection.system_role
        && !connection.system_prompt.trim().is_empty()
    {
        list.push(message(&connection.message_template, role, &connection.system_prompt));
    }

    for (speaker, text) in joined {
        let role = match speaker {
            Speaker::Evaluator => &connection.user_role,
            Speaker::Model => &connection.model_role,
        };

        list.push(message(&connection.message_template, role, &text));
    }

    Value::Array(list)
}

/// One message: the message template with `"{{role}}"` and
/// `"{{text}}"` replaced as values.
fn message(template: &Value, role: &str, text: &str) -> Value {
    match template {
        Value::String(placeholder) if placeholder == ROLE => Value::String(role.to_owned()),
        Value::String(placeholder) if placeholder == TEXT => Value::String(text.to_owned()),
        Value::Array(items) => Value::Array(items.iter().map(|item| message(item, role, text)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), message(value, role, text)))
                .collect(),
        ),
        other => other.clone(),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
