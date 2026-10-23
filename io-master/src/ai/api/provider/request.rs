//! The request to the AI service, made from the template of a
//! connection (`ai/api/connection.rs`) and the conversation.
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
use crate::ai::api::connection::Checked;
use crate::model::ai::{API_KEY, MESSAGES, MODEL, ROLE, SYSTEM, TEXT};
use crate::model::ai::{Speaker, Turn};
use crate::model::api_key::ApiKey;

/// What stands in place of the key where a request is shown.
pub const MASK: &str = "••••••••";

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
            (name.clone(), Zeroizing::new(value.replace(API_KEY, key).replace(MODEL, &connection.model)))
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
            fields.iter().map(|(name, value)| (name.clone(), fill(value, connection, messages))).collect(),
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
        Value::Object(fields) => {
            Value::Object(fields.iter().map(|(name, value)| (name.clone(), message(value, role, text))).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `request.rs`.

    use super::*;

    use serde_json::json;

    use crate::model::ai::Connection;

    fn evaluator(text: &str) -> Turn {
        Turn { speaker: Speaker::Evaluator, text: text.to_owned() }
    }

    fn model(text: &str) -> Turn {
        Turn { speaker: Speaker::Model, text: text.to_owned() }
    }

    fn key() -> ApiKey {
        ApiKey::new("sk-test-secret".to_owned()).unwrap()
    }

    /// A connection in the form of the template, with a key in a header.
    fn connection() -> Checked {
        Connection {
            name: "Test".to_owned(),
            model: "test-model".to_owned(),
            headers: "authorization: Bearer {{api_key}}\nx-model: {{model}}".to_owned(),
            system_prompt: "Be brief.".to_owned(),
            ..Connection::template()
        }
        .check()
        .unwrap()
    }

    /// A connection that takes the instructions in a field of its own
    /// and needs no key.
    fn system_field() -> Checked {
        Connection {
            name: "Test".to_owned(),
            model: "test-model".to_owned(),
            headers: String::new(),
            body: r#"{"model": "{{model}}", "system": "{{system}}", "max_tokens": 100, "messages": "{{messages}}"}"#
                .to_owned(),
            system_role: String::new(),
            system_prompt: "Be brief.".to_owned(),
            ..Connection::template()
        }
        .check()
        .unwrap()
    }

    #[test]
    fn the_placeholders_become_values() {
        let request = request(&system_field(), &[evaluator("Hi")], None).unwrap();

        assert_eq!(
            request.body,
            json!({
                "model": "test-model",
                "system": "Be brief.",
                "max_tokens": 100,
                "messages": [{"role": "user", "content": "Hi"}]
            })
        );
        assert_eq!(request.url, "http://127.0.0.1:11434/v1/chat/completions");
    }

    #[test]
    fn the_instructions_go_first_when_they_have_a_role() {
        let request = request(&connection(), &[evaluator("Hi")], Some(&key())).unwrap();

        assert_eq!(
            request.body["messages"],
            json!([{"role": "system", "content": "Be brief."}, {"role": "user", "content": "Hi"}])
        );
    }

    #[test]
    fn the_key_goes_only_into_its_header() {
        let request = request(&connection(), &[evaluator("Hi")], Some(&key())).unwrap();

        let headers: Vec<(&str, &str)> =
            request.headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
        assert_eq!(headers, [("authorization", "Bearer sk-test-secret"), ("x-model", "test-model")]);
        assert!(!request.body.to_string().contains("sk-test-secret"));
        assert!(!request.url.contains("sk-test-secret"));
    }

    #[test]
    fn a_needed_key_that_is_missing_is_an_error() {
        assert_eq!(request(&connection(), &[evaluator("Hi")], None).unwrap_err(), AiError::NoKey);

        // A connection without {{api_key}} needs none.
        assert!(request(&system_field(), &[evaluator("Hi")], None).is_ok());
    }

    #[test]
    fn the_preview_masks_the_key() {
        let preview = preview(&connection(), &[evaluator("Hi")]);

        assert_eq!(preview.headers[0].1.as_str(), format!("Bearer {MASK}"));
    }

    #[test]
    fn a_message_never_changes_the_structure_of_the_body() {
        let tricky = r#"", "model": "other", "x": {"#;
        let request = request(&system_field(), &[evaluator(tricky)], None).unwrap();

        assert_eq!(request.body["model"], "test-model");
        assert_eq!(request.body["messages"][0]["content"], tricky);
        assert!(request.body.get("x").is_none());

        // A message that looks like a placeholder stays text.
        let request = super::request(&system_field(), &[evaluator("{{system}}")], None).unwrap();
        assert_eq!(request.body["messages"][0]["content"], "{{system}}");
    }

    #[test]
    fn turns_become_alternating_messages() {
        let turns = [
            model("a greeting the model never got"),
            evaluator("first"),
            evaluator("second, after a failed reply"),
            model("   "),
            model("answer"),
            evaluator("next"),
        ];

        let request = request(&system_field(), &turns, None).unwrap();

        assert_eq!(
            request.body["messages"],
            json!([
                {"role": "user", "content": "first\n\nsecond, after a failed reply"},
                {"role": "assistant", "content": "answer"},
                {"role": "user", "content": "next"},
            ])
        );
    }

    #[test]
    fn the_roles_come_from_the_connection() {
        let connection = Connection {
            name: "Test".to_owned(),
            model: "m".to_owned(),
            user_role: "human".to_owned(),
            model_role: "bot".to_owned(),
            system_role: String::new(),
            ..Connection::template()
        }
        .check()
        .unwrap();

        let request = request(&connection, &[evaluator("a"), model("b")], None).unwrap();

        assert_eq!(
            request.body["messages"],
            json!([{"role": "human", "content": "a"}, {"role": "bot", "content": "b"}])
        );
    }

    #[test]
    fn the_fields_keep_the_order_of_the_template() {
        let request = request(&system_field(), &[evaluator("Hi")], None).unwrap();

        let names: Vec<&str> = request.body.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(names, ["model", "system", "max_tokens", "messages"]);
    }

    #[test]
    fn every_message_is_made_from_the_message_template() {
        let connection = Connection {
            name: "Parts".to_owned(),
            model: "m".to_owned(),
            message_template: r#"{"role": "{{role}}", "parts": [{"text": "{{text}}"}]}"#.to_owned(),
            user_role: "user".to_owned(),
            model_role: "model".to_owned(),
            system_role: String::new(),
            ..Connection::template()
        }
        .check()
        .unwrap();

        let request = request(&connection, &[evaluator("a \"quoted\" {{role}}"), model("b")], None).unwrap();

        assert_eq!(
            request.body["messages"],
            json!([
                {"role": "user", "parts": [{"text": "a \"quoted\" {{role}}"}]},
                {"role": "model", "parts": [{"text": "b"}]},
            ])
        );
    }
}
