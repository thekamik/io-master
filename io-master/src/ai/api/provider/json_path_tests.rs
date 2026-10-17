//! Unit tests of `json_path.rs`.

use super::*;

use serde_json::json;

fn path(text: &str) -> JsonPath {
    JsonPath::parse(text).unwrap()
}

#[test]
fn fields_and_positions_are_followed() {
    let answer = json!({"choices": [{"delta": {"content": "Hel"}}, {"delta": {"content": "other"}}]});

    assert_eq!(path("choices[0].delta.content").strings(&answer).as_deref(), Some("Hel"));
    assert_eq!(path("choices[1].delta.content").strings(&answer).as_deref(), Some("other"));
    assert_eq!(path("choices[2].delta.content").strings(&answer), None);
}

#[test]
fn every_element_is_joined() {
    // Blocks of another kind have no "text" and are passed over.
    let answer = json!({"content": [{"type": "thinking", "thinking": "hm"}, {"text": "Hello "}, {"text": "there"}]});

    assert_eq!(path("content[*].text").strings(&answer).as_deref(), Some("Hello there"));
}

#[test]
fn an_answer_that_is_a_list_can_be_read() {
    let answer = json!([{"generated_text": "Hi"}]);

    assert_eq!(path("[0].generated_text").strings(&answer).as_deref(), Some("Hi"));
}

#[test]
fn values_that_are_not_text_are_passed_over() {
    let answer = json!({"delta": {"content": null}, "usage": {"tokens": 12}});

    assert_eq!(path("delta.content").strings(&answer), None);
    assert_eq!(path("usage.tokens").number(&answer), Some(12));
    assert_eq!(path("usage.tokens").strings(&answer), None);
}

#[test]
fn a_path_on_the_wrong_kind_of_value_finds_nothing() {
    let answer = json!({"choices": {"0": "an object, not a list"}});

    assert!(path("choices[0]").find(&answer).is_empty());
    assert!(path("choices.0.x").find(&answer).is_empty());
}

#[test]
fn broken_paths_are_refused() {
    assert_eq!(JsonPath::parse("  "), Err(InvalidPath::Empty));
    assert_eq!(JsonPath::parse(&"a".repeat(MAX_LENGTH + 1)), Err(InvalidPath::TooLong));

    for broken in ["a..b", ".a", "a.", "a[", "a[x]", "a[1", "a]", "a[1]b", "a b", "a.[0]", "a\"b"] {
        assert!(matches!(JsonPath::parse(broken), Err(InvalidPath::Malformed(_))), "{broken} should be refused");
    }
}

#[test]
fn the_path_is_kept_as_written() {
    assert_eq!(path(" choices[0].delta.content ").as_str(), "choices[0].delta.content");
}
