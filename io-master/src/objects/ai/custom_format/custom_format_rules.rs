//! The rules of a custom format (`CustomFormat`): both its JSON texts are
//! JSON, each placeholder the program needs stands where it can be used, and
//! the places of a response are found once, as paths into its JSON.
//!
//! ```text
//! request   ──► check_request: JSON; "<prompt>" or "<messages>" in it; a placeholder of a list or of a
//!               number alone as a value
//! response  ──► places_of: JSON; "<response>" once; each placeholder alone as a value
//!               ──► Places: the path of each ("/result/output")
//!
//! text_leaves(JSON) ──► every text in it with its path, walked with a bounded stack (no recursion)
//! ```

use serde_json::Value;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::custom_format::custom_format_model::{CustomFormat, ResponseKind};
use crate::objects::ai::custom_format::custom_format_store::{
    MAX_FORMAT_JSON, MAX_FORMATS, MAX_KEY_HEADER, create_custom_format, read_custom_format, read_custom_formats,
    update_custom_format,
};
use crate::objects::error::DomainError;
use crate::objects::text::checked_text;

/// The placeholders of a request.
pub const MODEL: &str = "<model>";
pub const SYSTEM: &str = "<system>";
pub const PROMPT: &str = "<prompt>";
pub const MESSAGES: &str = "<messages>";
pub const MAX_ANSWER_TOKENS: &str = "<max_answer_tokens>";
pub const TEMPERATURE: &str = "<temperature>";

/// The placeholders of a request that are no text (a list, a number): each
/// stands alone as a value, never inside a longer text.
const WHOLE_VALUES: [&str; 3] = [MESSAGES, MAX_ANSWER_TOKENS, TEMPERATURE];

/// The placeholders of a response.
pub const RESPONSE: &str = "<response>";
pub const ERROR: &str = "<error>";
pub const INPUT_TOKENS: &str = "<input_tokens>";
pub const OUTPUT_TOKENS: &str = "<output_tokens>";

/// The placeholders of a response, in the order of [`Places`].
const RESPONSE_PLACEHOLDERS: [&str; 5] = [RESPONSE, ERROR, MODEL, INPUT_TOKENS, OUTPUT_TOKENS];

/// The most values of one JSON of a format: a bound of the walk through it
/// (rule 2), far above what [`MAX_FORMAT_JSON`] bytes hold.
const MAX_VALUES: usize = MAX_FORMAT_JSON;

/// The longest name of a format, in characters.
const MAX_NAME: usize = 200;

/// What a new format starts from: the smallest shapes that work, the key in
/// the header most services read.
const NEW_REQUEST: &str = "{\n  \"model\": \"<model>\",\n  \"system\": \"<system>\",\n  \"prompt\": \"<prompt>\"\n}";
const NEW_RESPONSE: &str = "{\n  \"response\": \"<response>\"\n}";
const NEW_KEY_HEADER: &str = "Authorization";
const NEW_KEY_PREFIX: &str = "Bearer ";

/// Where the JSON of a response holds what the program reads: the path of
/// each placeholder (a JSON Pointer, `"/result/output"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    /// The text of the answer (`<response>`).
    pub response: String,
    /// What the service says when it fails (`<error>`).
    pub error: Option<String>,
    /// The model that answered (`<model>`).
    pub model: Option<String>,
    /// The tokens the service counted of the question (`<input_tokens>`).
    pub input_tokens: Option<String>,
    /// The tokens it counted of the answer (`<output_tokens>`).
    pub output_tokens: Option<String>,
}

/// The JSON `text` of a format read: an object or a list.
///
/// Input: `text`, the request or the response of a format; `what`, its name
/// for the refusal ("the request").
/// Output: the JSON; `DomainError::Refused` for a text over
/// [`MAX_FORMAT_JSON`], that is no JSON, or that is a bare value.
///
/// Called by: [`check_request`], [`places_of`];
/// `objects/ai/custom_format/custom_format_ai.rs`: `custom_body`.
pub fn json_of(text: &str, what: &str) -> Result<Value, DomainError> {
    ensure!(text.len() <= MAX_FORMAT_JSON, DomainError::Refused(format!("{what} is over {MAX_FORMAT_JSON} bytes")));

    let value: Value =
        serde_json::from_str(text).map_err(|error| DomainError::Refused(format!("{what} is no JSON: {error}")))?;

    ensure!(
        value.is_object() || value.is_array(),
        DomainError::Refused(format!("{what} is neither an object nor a list of JSON"))
    );

    Ok(value)
}

/// Every text of the JSON `value` with its path, in no order: the values
/// that may hold a placeholder. The tree is walked with a stack, never by
/// recursion (rule 1).
///
/// Input: `value`, the JSON of a request or of a response.
/// Output: the paths and the texts (`[("/input/text", "<prompt>")]`), a key
/// written by RFC 6901 (`~` as `~0`, `/` as `~1`); `DomainError::Refused`
/// for more than [`MAX_VALUES`] values.
///
/// Called by: [`check_request`], [`places_of`];
/// `objects/ai/custom_format/custom_format_ai.rs`: `custom_body`.
pub fn text_leaves(value: &Value) -> Result<Vec<(String, String)>, DomainError> {
    let mut stack = vec![(String::new(), value)];
    let mut leaves = Vec::new();

    for _ in 0..MAX_VALUES {
        let Some((path, current)) = stack.pop() else {
            return Ok(leaves);
        };

        match current {
            Value::String(text) => leaves.push((path, text.clone())),
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    stack.push((format!("{path}/{index}"), item));
                }
            }
            Value::Object(fields) => {
                for (key, item) in fields {
                    stack.push((format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")), item));
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    Err(DomainError::Refused(format!("a JSON of more than {MAX_VALUES} values")))
}

/// Refuses a request the program could not ask a question with: no JSON, no
/// place for the question, or a placeholder of a list or of a number inside
/// a longer text.
///
/// Input: `request`, the request of a format.
/// Output: nothing when it is good; otherwise `DomainError::Refused` with
/// the reason.
///
/// Called by: [`checked`].
/// Calls: [`json_of`], [`text_leaves`].
pub fn check_request(request: &str) -> Result<(), DomainError> {
    let mut asks = false;

    for (path, text) in text_leaves(&json_of(request, "the request")?)? {
        asks = asks || text.contains(PROMPT) || text == MESSAGES;

        for placeholder in WHOLE_VALUES {
            ensure!(
                text == placeholder || !text.contains(placeholder),
                DomainError::Refused(format!(
                    "the request: {placeholder} at {path} stands inside a longer text; it is a whole value"
                ))
            );
        }
    }

    ensure!(asks, DomainError::Refused(format!("the request has no place for the question: {PROMPT} or {MESSAGES}")));

    Ok(())
}

/// The places of a response: the path of each of its placeholders.
///
/// Input: `response`, the response of a format.
/// Output: the places (`Places { response: "/result/output", .. }`);
/// `DomainError::Refused` for a text that is no JSON, without `<response>`,
/// with a placeholder twice or inside a longer text.
///
/// Called by: [`checked`]; `objects/ai/custom_format/custom_format_ai.rs`: `custom_exchange`.
/// Calls: [`json_of`], [`text_leaves`].
pub fn places_of(response: &str) -> Result<Places, DomainError> {
    let mut found: [Option<String>; 5] = [None, None, None, None, None];

    for (path, text) in text_leaves(&json_of(response, "the response")?)? {
        for (place, placeholder) in found.iter_mut().zip(RESPONSE_PLACEHOLDERS) {
            if text == placeholder {
                ensure!(
                    place.is_none(),
                    DomainError::Refused(format!("the response: {placeholder} stands twice; once is read"))
                );
                *place = Some(path.clone());
            } else {
                ensure!(
                    !text.contains(placeholder),
                    DomainError::Refused(format!(
                        "the response: {placeholder} at {path} stands inside a longer text; it is a whole value"
                    ))
                );
            }
        }
    }

    let [response, error, model, input_tokens, output_tokens] = found;
    let response =
        response.ok_or_else(|| DomainError::Refused(format!("the response has no place of the answer: {RESPONSE}")))?;

    Ok(Places { response, error, model, input_tokens, output_tokens })
}

/// A new format, not stored yet: the smallest request and response that
/// work, to be changed into those of the service.
///
/// Called by: `panels/views/settings/custom_formats.rs`: `custom_format_new`.
pub fn new_format() -> CustomFormat {
    CustomFormat {
        id: 0,
        name: String::new(),
        request: NEW_REQUEST.to_owned(),
        response: NEW_RESPONSE.to_owned(),
        response_kind: ResponseKind::Whole,
        key_header: NEW_KEY_HEADER.to_owned(),
        key_prefix: NEW_KEY_PREFIX.to_owned(),
    }
}

/// The format `format` as it is kept: its name trimmed, both its JSON texts
/// checked and kept as written, the header of its key one a request can
/// carry.
///
/// Input: `format`, a format as the page sends it.
/// Output: the format to store; `DomainError::Refused` for a name refused,
/// the refusals of [`check_request`] and [`places_of`], a header of the key
/// that is no name of a header, or signs before the key a header cannot
/// carry.
///
/// Called by: [`add_format`], [`change_format`].
/// Calls: `objects/text.rs`: `checked_text`; [`check_request`], [`places_of`].
fn checked(format: &CustomFormat) -> Result<CustomFormat, DomainError> {
    let name = checked_text(&format.name, "the name of the format", MAX_NAME)?;
    let key_header = format.key_header.trim();

    check_request(&format.request)?;
    places_of(&format.response)?;
    ensure!(
        !key_header.is_empty()
            && key_header.len() <= MAX_KEY_HEADER
            && key_header.chars().all(|character| character.is_ascii_alphanumeric() || character == '-'),
        DomainError::Refused("the header of the key: letters, digits and \"-\" (\"Authorization\")".to_owned())
    );
    ensure!(
        format.key_prefix.len() <= MAX_KEY_HEADER
            && format.key_prefix.chars().all(|character| character == ' ' || character.is_ascii_graphic()),
        DomainError::Refused("what stands before the key: plain signs of ASCII (\"Bearer \")".to_owned())
    );

    Ok(CustomFormat {
        id: format.id,
        name,
        request: format.request.clone(),
        response: format.response.clone(),
        response_kind: format.response_kind,
        key_header: key_header.to_owned(),
        key_prefix: format.key_prefix.clone(),
    })
}

/// Stores the new format `format`.
///
/// Input: `format`, the format typed, its id 0.
/// Output: the id of the new format (`2`); the refusals of [`checked`],
/// `DomainError::Refused` for [`MAX_FORMATS`] formats already, an error of
/// the database (a name taken).
///
/// Called by: `panels/views/settings/custom_formats.rs`: `custom_format_save`.
/// Calls: [`checked`]; `objects/ai/custom_format/custom_format_store.rs`: `read_custom_formats`,
/// `create_custom_format`.
pub fn add_format(db: &UserDatabase, format: &CustomFormat) -> Result<i64, DomainError> {
    let format = checked(format)?;

    ensure!(
        read_custom_formats(db)?.len() < MAX_FORMATS,
        DomainError::Refused(format!("{MAX_FORMATS} formats already"))
    );

    Ok(create_custom_format(db, &format)?)
}

/// Writes the stored format `format.id` anew. Every integration that names
/// it speaks by the new texts from its next question on.
///
/// Input: `format`, the format as changed, its id that of a stored format.
/// Output: nothing; the refusals of [`checked`], an error of the database
/// for a format missing or a name taken.
///
/// Called by: `panels/views/settings/custom_formats.rs`: `custom_format_save`.
/// Calls: [`checked`]; `objects/ai/custom_format/custom_format_store.rs`: `read_custom_format`,
/// `update_custom_format`.
pub fn change_format(db: &UserDatabase, format: &CustomFormat) -> Result<(), DomainError> {
    let format = checked(format)?;

    read_custom_format(db, format.id)?;

    Ok(update_custom_format(db, &format)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the rules of a custom format: the placeholders found by
    //! their paths, a format that cannot ask or be read refused.

    use super::*;

    use crate::test_support::database::{custom_format, database};

    #[test]
    fn the_places_of_a_response_are_the_paths_of_its_placeholders() {
        let response = r#"{"result": {"outputs": [{"te/xt": "<response>"}], "usage": {"in": "<input_tokens>",
            "out": "<output_tokens>"}}, "engine": "<model>", "fault": "<error>", "version": "2.1", "ok": true}"#;

        assert_eq!(
            places_of(response).unwrap(),
            Places {
                response: "/result/outputs/0/te~1xt".to_owned(),
                error: Some("/fault".to_owned()),
                model: Some("/engine".to_owned()),
                input_tokens: Some("/result/usage/in".to_owned()),
                output_tokens: Some("/result/usage/out".to_owned()),
            }
        );
        assert_eq!(places_of(r#"["<response>"]"#).unwrap().response, "/0");

        for refused in [
            r#"{"text": "no placeholder"}"#,
            r#"{"a": "<response>", "b": "<response>"}"#,
            r#"{"a": "The answer: <response>"}"#,
            r#""<response>""#,
            "{no json}",
        ] {
            assert!(matches!(places_of(refused), Err(DomainError::Refused(_))), "{refused}");
        }
    }

    #[test]
    fn a_request_needs_a_place_for_the_question() {
        for good in [
            r#"{"input": "<prompt>"}"#,
            r#"{"input": "System: <system>\nUser: <prompt>", "n": "<max_answer_tokens>", "t": "<temperature>"}"#,
            r#"{"chat": "<messages>", "own": {"top_k": 40, "tags": ["<lab>"]}}"#,
        ] {
            check_request(good).unwrap();
        }

        for refused in [
            r#"{"model": "<model>"}"#,
            r#"{"input": "<prompt>", "chat": "history: <messages>"}"#,
            r#"{"input": "<prompt>", "n": "at most <max_answer_tokens>"}"#,
            r#"{"input": "<prompt>""#,
        ] {
            assert!(matches!(check_request(refused), Err(DomainError::Refused(_))), "{refused}");
        }
    }

    #[test]
    fn a_format_is_added_and_changed_only_when_it_holds_together() {
        let (_directory, db) = database();
        let id = add_format(&db, &CustomFormat { name: " The institute ".to_owned(), ..custom_format() }).unwrap();
        let stored = read_custom_format(&db, id).unwrap();

        assert_eq!(stored.name, "The institute");
        assert_eq!(stored.request, custom_format().request);

        change_format(
            &db,
            &CustomFormat { key_header: " X-Api-Key ".to_owned(), key_prefix: String::new(), ..stored.clone() },
        )
        .unwrap();
        assert_eq!(read_custom_format(&db, id).unwrap().key_header, "X-Api-Key");

        for refused in [
            CustomFormat { name: " ".to_owned(), ..stored.clone() },
            CustomFormat { request: "{}".to_owned(), ..stored.clone() },
            CustomFormat { response: "{}".to_owned(), ..stored.clone() },
            CustomFormat { key_header: "Bad Header:".to_owned(), ..stored.clone() },
            CustomFormat { key_prefix: "Bearer\n".to_owned(), ..stored.clone() },
        ] {
            assert!(matches!(change_format(&db, &refused), Err(DomainError::Refused(_))));
        }

        assert!(change_format(&db, &CustomFormat { id: 99, ..stored }).is_err());
        assert!(places_of(&new_format().response).is_ok() && check_request(&new_format().request).is_ok());
    }
}
