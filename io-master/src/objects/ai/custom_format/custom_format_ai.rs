//! The custom format at work: the JSON of a question made from the request
//! of the format, and the answer of the service read by its response.
//!
//! ```text
//! custom_body:      request ──► each placeholder replaced: a text inside its text, a list or a number as the
//!                               whole value ──► the body sent
//! custom_exchange:  response ──► Places ──► CustomExchange (with the header of the key and the kind)
//! read_whole:       the whole answer ──► JSON ──► read_answer ──► Reading (complete)
//! read_custom_event: one event of a stream ──► JSON ──► read_answer ──► the next piece of the text
//! tried:            a sample of an answer read as the service's would be, without any connection
//! ```

use serde_json::{Value, json};

use crate::objects::ai::chat::chat_model::Speaker;
use crate::objects::ai::custom_format::custom_format_model::{CustomFormat, ResponseKind};
use crate::objects::ai::custom_format::custom_format_rules::{
    MAX_ANSWER_TOKENS, MESSAGES, MODEL, PROMPT, Places, SYSTEM, TEMPERATURE, json_of, places_of, text_leaves,
};
use crate::objects::ai::integration::integration_ai::reading::Reading;
use crate::objects::ai::integration::integration_ai::sse::SseEvent;
use crate::objects::ai::integration::integration_ai::turns::Turn;
use crate::objects::ai::journal::journal_model::Ending;
use crate::objects::ai_error::{AiError, shorten};

/// What ends a stream of events at many services, in place of a JSON.
const STREAM_END: &str = "[DONE]";

/// What a sample of a question is made of, to show a request filled.
const SAMPLE_MODEL: &str = "the-model-of-the-integration";
const SAMPLE_SYSTEM: &str = "The instructions of the agent, its knowledge and its task.";
const SAMPLE_PROMPT: &str = "The question.";
const SAMPLE_ANSWER_TOKENS: u32 = 16_000;

/// How a custom service is spoken with, made once from its format for one
/// question: where its key goes, how its answer comes, where it is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomExchange {
    /// The header of the API key ("Authorization").
    pub key_header: String,
    /// What stands before the key in it ("Bearer ").
    pub key_prefix: String,
    /// Whether the answer comes as a stream of events.
    pub events: bool,
    /// Where the JSON of the answer holds what is read.
    pub places: Places,
}

/// The text `text` of a request with the placeholders of `values` replaced,
/// in one pass: a placeholder that comes inside a value (a document that
/// writes "<system>") is never replaced again.
///
/// Input: `text`, a text of a request; `values`, the placeholders with
/// their values (`[("<prompt>", "Assess it.")]`).
/// Output: the text ("User: Assess it.").
///
/// Called by: [`custom_body`].
fn replaced(text: &str, values: &[(&str, &str)]) -> String {
    let mut written = String::with_capacity(text.len());
    let mut rest = text;

    // Each pass takes at least one byte of the text.
    for _ in 0..=text.len() {
        let Some(start) = rest.find('<') else {
            break;
        };
        let (before, from) = rest.split_at(start);
        let mut value = "<";
        let mut after = from.get(1..).unwrap_or_default();

        for (placeholder, given) in values {
            if let Some(tail) = from.strip_prefix(placeholder) {
                value = given;
                after = tail;
            }
        }

        written.push_str(before);
        written.push_str(value);
        rest = after;
    }

    written.push_str(rest);
    written
}

/// The body of a question in the custom format `format`: its request with
/// every placeholder replaced, everything else as the user wrote it.
///
/// Input:
/// - `format`: the format of the integration, checked when it was stored.
/// - `model`: the model of the integration.
/// - `system`: the instructions; may be empty.
/// - `turns`: the conversation, its last turn the question of the user.
/// - `answer_tokens`: the longest answer asked for.
/// - `temperature`: the temperature of the agent in hundredths; `None` is
///   written `null`.
///
/// Output: the body as JSON (`{"engine": "m", "input": "Assess it."}`);
/// `AiError::Refused` for a request that is no JSON any more or holds a
/// placeholder of a list inside a text (a database the program did not
/// write), `AiError::NoQuestion` without a turn.
///
/// Called by: `objects/ai/integration/integration_ai/protocol.rs`: `body`; [`tried`].
/// Calls: `objects/ai/custom_format/custom_format_rules.rs`: `json_of`, `text_leaves`; [`replaced`].
pub fn custom_body(
    format: &CustomFormat,
    model: &str,
    system: &str,
    turns: &[Turn],
    answer_tokens: u32,
    temperature: Option<u32>,
) -> Result<Value, AiError> {
    let prompt = turns.last().ok_or(AiError::NoQuestion)?.text.as_str();
    let texts = [(MODEL, model), (SYSTEM, system), (PROMPT, prompt)];
    let mut messages = Vec::with_capacity(turns.len());

    for turn in turns {
        let role = match turn.speaker {
            Speaker::User => "user",
            Speaker::Ai => "assistant",
        };

        messages.push(json!({"role": role, "content": turn.text}));
    }

    let mut body = json_of(&format.request, "the request")?;

    for (path, text) in text_leaves(&body)? {
        // A list and the numbers take the place of the whole value; a text
        // is written into the text around its placeholder.
        let value = match text.as_str() {
            MESSAGES => Value::Array(messages.clone()),
            MAX_ANSWER_TOKENS => json!(answer_tokens),
            TEMPERATURE => temperature.map_or(Value::Null, |hundredths| json!(f64::from(hundredths) / 100.0)),
            _ => Value::String(replaced(&text, &texts)),
        };
        let place = body
            .pointer_mut(&path)
            .ok_or_else(|| AiError::Refused(format!("the request of the format has no value at {path}")))?;

        *place = value;
    }

    Ok(body)
}

/// How the service of the format `format` is spoken with.
///
/// Input: `format`, the format of the integration.
/// Output: the exchange; `AiError::Domain` when its response cannot be read
/// by (a database the program did not write).
///
/// Called by: `objects/ai/integration/integration_ai/question.rs`: `prepare`; [`tried`].
/// Calls: `objects/ai/custom_format/custom_format_rules.rs`: `places_of`.
pub fn custom_exchange(format: &CustomFormat) -> Result<CustomExchange, AiError> {
    Ok(CustomExchange {
        key_header: format.key_header.clone(),
        key_prefix: format.key_prefix.clone(),
        events: format.response_kind == ResponseKind::Events,
        places: places_of(&format.response)?,
    })
}

/// The count of tokens the answer `value` holds at `path`, when the format
/// names such a place and the answer has a number there.
///
/// Input: `value`, the JSON of an answer; `path`, a place of the format.
/// Output: the count (`Some(812)`); `None` without the place or the number;
/// `AiError::UnexpectedAnswer` for a number that is no count of tokens.
///
/// Called by: [`read_answer`].
fn tokens_at(value: &Value, path: Option<&String>) -> Result<Option<u32>, AiError> {
    let Some(number) = path.and_then(|path| value.pointer(path)).filter(|found| found.is_number()) else {
        return Ok(None);
    };
    let count = number
        .as_u64()
        .and_then(|count| u32::try_from(count).ok())
        .ok_or_else(|| AiError::UnexpectedAnswer(format!("{number} is no count of tokens")))?;

    Ok(Some(count))
}

/// Lays the JSON `value` of an answer over the response of the format: what
/// stands at the places of its placeholders is read into `reading`.
///
/// Input: `value`, the JSON of a whole answer or of one event; `places`,
/// the places of the format; `reading`, what was read so far.
/// Output: the text at the place of `<response>` (`Some("PASS")`), added
/// to `reading`; `None` when the JSON has no text there (an event without
/// a piece). `AiError::Reported` with what stands at `<error>`,
/// `AiError::UnexpectedAnswer` for a value at `<response>` that is no text.
///
/// Called by: [`read_whole`], [`read_custom_event`].
/// Calls: [`tokens_at`]; `objects/ai_error.rs`: `shorten`.
fn read_answer(value: &Value, places: &Places, reading: &mut Reading) -> Result<Option<String>, AiError> {
    // An error is what the service wrote there, when it wrote anything: a
    // text, or its own JSON.
    match places.error.as_ref().and_then(|path| value.pointer(path)) {
        None | Some(Value::Null | Value::Bool(false)) => {}
        Some(Value::String(said)) if said.is_empty() => {}
        Some(Value::String(said)) => return Err(AiError::Reported(shorten(said))),
        Some(other) => return Err(AiError::Reported(shorten(&other.to_string()))),
    }

    if reading.model.is_none() {
        reading.model =
            places.model.as_ref().and_then(|path| value.pointer(path)).and_then(Value::as_str).map(str::to_owned);
    }

    if let Some(count) = tokens_at(value, places.input_tokens.as_ref())? {
        reading.input_tokens = count;
    }

    if let Some(count) = tokens_at(value, places.output_tokens.as_ref())? {
        reading.output_tokens = count;
    }

    match value.pointer(&places.response) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            reading.text.push_str(text);
            Ok(Some(text.clone()))
        }
        Some(_) => Err(AiError::UnexpectedAnswer(format!(
            "the answer holds no text at {}, where the format has {}",
            places.response, "<response>"
        ))),
    }
}

/// Reads the whole answer of a custom service: one JSON document, laid over
/// the response of its format. An answer that came whole is complete.
///
/// Input: `bytes`, the body of the answer, whole; `exchange`, of the
/// format; `reading`, empty.
/// Output: the text of the answer (`"PASS"`), also in `reading`, which is
/// done; `AiError::UnexpectedAnswer` for a body that is no JSON or has no
/// text where the format has `<response>`, the errors of [`read_answer`].
///
/// Called by: `objects/ai/integration/integration_ai/connection.rs`: `read_whole_answer`; [`tried`].
/// Calls: [`read_answer`].
pub fn read_whole(bytes: &[u8], exchange: &CustomExchange, reading: &mut Reading) -> Result<String, AiError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| AiError::UnexpectedAnswer(format!("an answer that is no JSON: {error}")))?;
    let text = read_answer(&value, &exchange.places, reading)?.ok_or_else(|| {
        AiError::UnexpectedAnswer(format!(
            "the answer has nothing at {}, where the format has <response>",
            exchange.places.response
        ))
    })?;

    reading.ending = Some(Ending::Complete);
    reading.done = true;

    Ok(text)
}

/// Reads one event of the stream of a custom service: a JSON document laid
/// over the response of its format, or the mark that ends the stream.
///
/// Input: `event`, an event of the stream; `exchange`, of the format;
/// `reading`, what was read so far.
/// Output: the next piece of the text, when the event carries one;
/// `reading.done` after the mark `[DONE]`. `AiError::UnexpectedAnswer` for
/// an event that is no JSON, the errors of [`read_answer`].
///
/// Called by: `objects/ai/integration/integration_ai/protocol.rs`: `read_event`; [`tried`].
/// Calls: [`read_answer`].
pub fn read_custom_event(
    event: &SseEvent,
    exchange: &CustomExchange,
    reading: &mut Reading,
) -> Result<Option<String>, AiError> {
    if event.data.trim() == STREAM_END {
        reading.ending = Some(Ending::Complete);
        reading.done = true;

        return Ok(None);
    }

    let value: Value = serde_json::from_str(&event.data)
        .map_err(|error| AiError::UnexpectedAnswer(format!("an event that is no JSON: {error}")))?;

    Ok(read_answer(&value, &exchange.places, reading)?.filter(|piece| !piece.is_empty()))
}

/// Tries the format `format` without any connection: its request filled
/// with a sample of a question, and the sample `sample` of an answer read as
/// the answer of the service would be.
///
/// Input: `format`, a format as typed, stored or not; `sample`, an answer
/// of the service copied from its documentation (one event of it for a
/// stream), or empty to try the request alone.
/// Output: the request as it would be sent, and what was read of the
/// sample (`("{\n  \"input\": \"The question.\"\n}", Some(reading))`); the
/// refusals of the format, the errors of the reading.
///
/// Called by: `panels/views/settings/custom_formats.rs`: `custom_format_try`.
/// Calls: [`custom_body`], [`custom_exchange`], [`read_whole`], [`read_custom_event`].
pub fn tried(format: &CustomFormat, sample: &str) -> Result<(String, Option<Reading>), AiError> {
    let turns = [Turn { speaker: Speaker::User, text: SAMPLE_PROMPT.to_owned() }];
    let body = custom_body(format, SAMPLE_MODEL, SAMPLE_SYSTEM, &turns, SAMPLE_ANSWER_TOKENS, None)?;
    let request = serde_json::to_string_pretty(&body).map_err(|error| AiError::Refused(error.to_string()))?;
    let exchange = custom_exchange(format)?;

    if sample.trim().is_empty() {
        return Ok((request, None));
    }

    let mut reading = Reading::default();

    if exchange.events {
        read_custom_event(&SseEvent { name: String::new(), data: sample.trim().to_owned() }, &exchange, &mut reading)?;
    } else {
        read_whole(sample.as_bytes(), &exchange, &mut reading)?;
    }

    Ok((request, Some(reading)))
}

#[cfg(test)]
mod tests {
    //! Tests of a custom format at work: the request filled, an answer and
    //! the events of a stream read by the response.

    use super::*;

    use crate::test_support::database::custom_format;

    fn turns() -> Vec<Turn> {
        vec![
            Turn { speaker: Speaker::User, text: "What is the TSF?".to_owned() },
            Turn { speaker: Speaker::Ai, text: "The security functionality.".to_owned() },
            Turn { speaker: Speaker::User, text: "Assess \"it\" <system>.".to_owned() },
        ]
    }

    #[test]
    fn the_request_is_filled_and_what_is_the_users_own_stays() {
        let format = CustomFormat {
            request: r#"{"engine": "<model>", "input": "S: <system> | U: <prompt>", "chat": "<messages>",
                "limits": {"n": "<max_answer_tokens>", "t": "<temperature>"}, "top_k": 40, "tags": ["<lab>", "a<b"]}"#
                .to_owned(),
            ..custom_format()
        };
        let body = custom_body(&format, "m1", "You assess.", &turns(), 900, Some(20)).unwrap();

        assert_eq!(body["engine"], "m1");
        // The placeholder inside the question is the text of the user, never replaced.
        assert_eq!(body["input"], "S: You assess. | U: Assess \"it\" <system>.");
        assert_eq!(body["chat"].as_array().unwrap().len(), 3);
        assert_eq!(body["chat"][1], json!({"role": "assistant", "content": "The security functionality."}));
        assert_eq!(body["limits"], json!({"n": 900, "t": 0.2}));
        assert_eq!(body["top_k"], 40);
        assert_eq!(body["tags"], json!(["<lab>", "a<b"]));

        let without = custom_body(&format, "m1", "", &turns(), 900, None).unwrap();

        assert_eq!(without["limits"]["t"], Value::Null);
        assert!(matches!(custom_body(&format, "m1", "", &[], 900, None), Err(AiError::NoQuestion)));
    }

    #[test]
    fn a_whole_answer_is_read_where_the_format_has_its_placeholders() {
        let format = CustomFormat {
            response:
                r#"{"result": {"output": "<response>", "usage": {"in": "<input_tokens>", "out": "<output_tokens>"}},
                "engine": "<model>", "fault": "<error>"}"#
                    .to_owned(),
            ..custom_format()
        };
        let exchange = custom_exchange(&format).unwrap();
        let mut reading = Reading::default();
        let answer = r#"{"result": {"output": "PASS", "usage": {"in": 812, "out": 3}, "more": [1, 2]},
            "engine": "institute-7", "fault": null, "trace": "abc"}"#;

        assert_eq!(read_whole(answer.as_bytes(), &exchange, &mut reading).unwrap(), "PASS");
        assert_eq!((reading.text.as_str(), reading.input_tokens, reading.output_tokens), ("PASS", 812, 3));
        assert_eq!(
            (reading.model.as_deref(), reading.ending, reading.done),
            (Some("institute-7"), Some(Ending::Complete), true)
        );

        let mut failed = Reading::default();

        assert!(matches!(
            read_whole(br#"{"fault": "the model is busy"}"#, &exchange, &mut failed),
            Err(AiError::Reported(said)) if said == "the model is busy"
        ));
        assert!(matches!(
            read_whole(br#"{"fault": {"code": 7}}"#, &exchange, &mut failed),
            Err(AiError::Reported(said)) if said == r#"{"code":7}"#
        ));

        for unexpected in [r#"{"result": {}}"#, r#"{"result": {"output": {"text": "PASS"}}}"#, "PASS", r#"{"result"#] {
            assert!(
                matches!(
                    read_whole(unexpected.as_bytes(), &exchange, &mut Reading::default()),
                    Err(AiError::UnexpectedAnswer(_))
                ),
                "{unexpected}"
            );
        }
    }

    #[test]
    fn the_events_of_a_stream_are_read_piece_by_piece() {
        let format = CustomFormat {
            response: r#"{"delta": "<response>", "out": "<output_tokens>"}"#.to_owned(),
            response_kind: ResponseKind::Events,
            ..custom_format()
        };
        let exchange = custom_exchange(&format).unwrap();
        let mut reading = Reading::default();
        let event = |data: &str| SseEvent { name: String::new(), data: data.to_owned() };

        assert_eq!(
            read_custom_event(&event(r#"{"delta": "PA"}"#), &exchange, &mut reading).unwrap().as_deref(),
            Some("PA")
        );
        assert_eq!(read_custom_event(&event(r#"{"status": "thinking"}"#), &exchange, &mut reading).unwrap(), None);
        assert_eq!(
            read_custom_event(&event(r#"{"delta": "SS", "out": 2}"#), &exchange, &mut reading).unwrap().as_deref(),
            Some("SS")
        );
        assert!(!reading.done);
        assert_eq!(read_custom_event(&event("[DONE]"), &exchange, &mut reading).unwrap(), None);
        assert_eq!((reading.text.as_str(), reading.output_tokens, reading.done), ("PASS", 2, true));
        assert!(matches!(
            read_custom_event(&event("no json"), &exchange, &mut reading),
            Err(AiError::UnexpectedAnswer(_))
        ));
    }

    #[test]
    fn a_format_is_tried_without_a_connection() {
        let (request, read) = tried(&custom_format(), r#"{"response": "It works."}"#).unwrap();

        assert!(request.contains("\"The question.\"") && request.contains(SAMPLE_MODEL));
        assert_eq!(read.unwrap().text, "It works.");
        assert!(tried(&custom_format(), " ").unwrap().1.is_none());
        assert!(tried(&custom_format(), r#"{"other": 1}"#).is_err());
        assert!(tried(&CustomFormat { request: "{}{".to_owned(), ..custom_format() }, "").is_err());
    }
}
