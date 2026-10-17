//! Unit tests of `field_help.rs`.

use super::*;

/// A connection with a key typed into a header by mistake, and a CA
/// certificate.
fn careless() -> Connection {
    Connection {
        name: "Lab".to_owned(),
        model: "qwen3".to_owned(),
        headers: "x-api-key: sk-typed-by-mistake\nauthorization: Bearer {{api_key}}".to_owned(),
        ca_certificate: "-----BEGIN CERTIFICATE-----\nMIIBsecret\n-----END CERTIFICATE-----".to_owned(),
        ..Connection::template()
    }
}

#[test]
fn every_field_has_an_instruction_with_its_title_and_the_closing() {
    for field in Field::ALL {
        let text = instruction(field, &careless());

        assert!(text.starts_with(CONTEXT), "{}", field.name());
        assert!(text.contains(&format!("The field: {}", explanation(field).title)), "{}", field.name());
        assert!(text.contains("The service I want to use:"), "{}", field.name());
        assert!(text.trim_end().ends_with(ANSWER.trim_end()), "{}", field.name());
    }
}

#[test]
fn no_value_of_a_header_and_no_certificate_is_shown() {
    for field in Field::ALL {
        let text = instruction(field, &careless());

        assert!(!text.contains("sk-typed-by-mistake"), "{}", field.name());
        assert!(!text.contains("MIIBsecret"), "{}", field.name());
    }

    let text = instruction(Field::Headers, &careless());
    assert!(text.contains("x-api-key: (value not shown)"));
    assert!(text.contains("authorization: (value not shown; uses {{api_key}})"));
}

#[test]
fn the_context_fits_the_field() {
    let request = instruction(Field::Body, &careless());
    assert!(request.contains("- Model: qwen3"));
    assert!(request.contains("- Message template: {\"role\": \"{{role}}\", \"content\": \"{{text}}\"}"));
    assert!(!request.contains("- Path to the text:"));

    let answer = instruction(Field::StopPath, &careless());
    assert!(answer.contains("- Path to the text: choices[0].delta.content"));
    assert!(answer.contains("- This field now: choices[0].finish_reason"));
    assert!(answer.contains("A path is written like this"));
    assert!(!answer.contains("- Body:"));
}

#[test]
fn an_empty_field_is_said_to_be_empty() {
    let connection = Connection { done_marker: String::new(), ..careless() };

    assert!(instruction(Field::DoneMarker, &connection).contains("- This field now: (empty)"));
}

#[test]
fn the_field_itself_is_not_repeated_in_the_context() {
    let text = instruction(Field::Body, &careless());

    // The body of the form is indented; the example is on one line.
    assert_eq!(text.matches("\n  \"stream\": true").count(), 1);
    assert!(!instruction(Field::Url, &careless()).contains("- Endpoint:"));
    assert!(!instruction(Field::TextPath, &careless()).contains("- Path to the text:"));
}

#[test]
fn headers_that_hold_no_secret_are_shown() {
    let connection = Connection { headers: "Content-Type: application/json\nx-secret: abc".to_owned(), ..careless() };

    let text = instruction(Field::Body, &connection);
    assert!(text.contains("Content-Type: application/json"));
    assert!(text.contains("x-secret: (value not shown)"));
}
