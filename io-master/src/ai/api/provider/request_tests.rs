//! Unit tests of `request.rs`.

use super::*;

use serde_json::json;

use crate::ai::api::connection::Connection;

fn evaluator(text: &str) -> Turn {
    Turn {
        speaker: Speaker::Evaluator,
        text: text.to_owned(),
    }
}

fn model(text: &str) -> Turn {
    Turn {
        speaker: Speaker::Model,
        text: text.to_owned(),
    }
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
        body: r#"{"model": "{{model}}", "system": "{{system}}", "max_tokens": 100, "messages": "{{messages}}"}"#.to_owned(),
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

    let headers: Vec<(&str, &str)> = request.headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
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
