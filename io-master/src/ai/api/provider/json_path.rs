//! Paths into a JSON answer: how the evaluator tells the program
//! where the text, the stop reason or an error is in the answer of
//! their AI service (`ai/api/connection.rs`).
//!
//! A path is a list of steps separated by dots; a step is the name
//! of a field, optionally followed by positions in a list:
//!
//! ```text
//! choices[0].delta.content    the field "content" of "delta" of the
//!                             first element of "choices"
//! content[*].text             "text" of every element of "content"
//! [0].generated_text          "generated_text" of the first element
//!                             of an answer that is a list
//! ```
//!
//! `[*]` stands for every element, so a path can find several
//! values; [`JsonPath::strings`] joins them, for example the text
//! blocks of an answer that also holds blocks of another kind.
//! A path that finds nothing is not an error here: the reader of the
//! answer (`ai/api/provider/response.rs`) decides what that means.

use std::fmt;

use serde_json::Value;

/// The longest path that is accepted, in characters.
pub const MAX_LENGTH: usize = 200;

/// One step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// A field of an object.
    Field(String),
    /// The element at this position of a list (counted from 0).
    Index(usize),
    /// Every element of a list.
    Every,
}

/// A checked path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPath {
    /// The path as it was written, to show it again.
    text: String,
    steps: Vec<Step>,
}

/// Why a text is not a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidPath {
    /// Nothing was written.
    Empty,
    /// Longer than [`MAX_LENGTH`].
    TooLong,
    /// The path is not written as described at the top of this
    /// file; holds what is wrong.
    Malformed(String),
}

impl fmt::Display for InvalidPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidPath::Empty => formatter.write_str("the path is empty"),
            InvalidPath::TooLong => write!(formatter, "the path is longer than {MAX_LENGTH} characters"),
            InvalidPath::Malformed(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for InvalidPath {}

impl JsonPath {
    /// Reads a path as it was written (spaces around it are
    /// removed).
    pub fn parse(text: &str) -> Result<JsonPath, InvalidPath> {
        let text = text.trim();

        if text.is_empty() {
            return Err(InvalidPath::Empty);
        }

        if text.chars().count() > MAX_LENGTH {
            return Err(InvalidPath::TooLong);
        }

        let mut steps = Vec::new();

        // Every part between dots: a name, then any number of
        // "[n]" or "[*]". Only the first part may have no name
        // ("[0].text": the answer is a list).
        for (number, part) in text.split('.').enumerate() {
            let (name, mut positions) = part.split_at(part.find('[').unwrap_or(part.len()));

            if name.is_empty() && (number > 0 || positions.is_empty()) {
                return Err(InvalidPath::Malformed(format!("the path \"{text}\" has an empty step")));
            }

            if !name.is_empty() {
                if !name.chars().all(|character| character.is_alphanumeric() || "_-$@".contains(character)) {
                    return Err(InvalidPath::Malformed(format!(
                        "the step \"{name}\" of the path holds a character a name of a field cannot have here"
                    )));
                }

                steps.push(Step::Field(name.to_owned()));
            }

            while !positions.is_empty() {
                let malformed = || InvalidPath::Malformed(format!("the step \"{part}\" has a broken position [..]"));

                let inside_end = positions.find(']').ok_or_else(malformed)?;
                let inside = positions.get(1..inside_end).ok_or_else(malformed)?;

                if !positions.starts_with('[') {
                    return Err(malformed());
                }

                steps.push(match inside {
                    "*" => Step::Every,
                    number => Step::Index(number.parse().map_err(|_| malformed())?),
                });

                positions = &positions[inside_end + 1..];
            }
        }

        Ok(JsonPath { text: text.to_owned(), steps })
    }

    /// The path as it was written.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Every value the path finds in `value`, in order.
    pub fn find<'a>(&self, value: &'a Value) -> Vec<&'a Value> {
        let mut found = vec![value];

        for step in &self.steps {
            found = found
                .into_iter()
                .flat_map(|value| -> Vec<&'a Value> {
                    match (step, value) {
                        (Step::Field(name), Value::Object(fields)) => fields.get(name).into_iter().collect(),
                        (Step::Index(index), Value::Array(items)) => items.get(*index).into_iter().collect(),
                        (Step::Every, Value::Array(items)) => items.iter().collect(),
                        _ => Vec::new(),
                    }
                })
                .collect();
        }

        found
    }

    /// The texts the path finds, joined; `None` when it finds no
    /// text. Values that are not text (`null` in the last piece of
    /// a stream, for example) are passed over.
    pub fn strings(&self, value: &Value) -> Option<String> {
        let texts: Vec<&str> = self.find(value).into_iter().filter_map(Value::as_str).collect();

        (!texts.is_empty()).then(|| texts.concat())
    }

    /// The first whole number the path finds (a token count).
    pub fn number(&self, value: &Value) -> Option<u64> {
        self.find(value).into_iter().find_map(Value::as_u64)
    }
}

#[cfg(test)]
mod tests {
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
        let answer =
            json!({"content": [{"type": "thinking", "thinking": "hm"}, {"text": "Hello "}, {"text": "there"}]});

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
}
