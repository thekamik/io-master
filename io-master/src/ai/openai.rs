//! The chat completions of OpenAI (`/v1/chat/completions`), which most
//! local servers offer as well (Ollama, llama.cpp, vLLM): the body of a
//! question, its header and the reading of the answer while it streams.
//!
//! ```text
//! the question   {"model", "stream": true, "stream_options": {"include_usage": true},
//!                 "messages": [{"role": "system"}, {"role": "user"}, {"role": "assistant"}, ...]}
//! the answer     data: {"model", "choices": [{"delta": {"content": "Hel"}}]}
//!                data: {"choices": [{"delta": {}, "finish_reason": "stop"}]}
//!                data: {"choices": [], "usage": {"prompt_tokens", "completion_tokens"}}
//!                data: [DONE]
//! ```
//!
//! A local server needs no key; the key, when there is one, goes into the
//! header `authorization: Bearer ...`.

use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::ai::conversation::Turn;
use crate::ai::error::{AiError, shorten};
use crate::ai::reading::Reading;
use crate::ai::sse::SseEvent;
use crate::model::ai::chat::Speaker;
use crate::model::ai::journal::Ending;

/// The body of a question: the instructions as the first message of the
/// role `system`, then the turns.
///
/// Called by: `ask.rs` ([`crate::ai::ask::prepare`]).
pub fn body(model: &str, system: &str, turns: &[Turn]) -> Value {
    let mut messages = Vec::with_capacity(turns.len() + 1);

    if !system.trim().is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }

    for turn in turns {
        let role = match turn.speaker {
            Speaker::User => "user",
            Speaker::Ai => "assistant",
        };

        messages.push(json!({"role": role, "content": turn.text}));
    }

    json!({"model": model, "stream": true, "stream_options": {"include_usage": true}, "messages": messages})
}

/// The header of the key, when there is one: `authorization`, the value
/// `Bearer <key>`.
///
/// Called by: `client.rs` (the request).
pub fn key_header(key: &str) -> (&'static str, Zeroizing<String>) {
    ("authorization", Zeroizing::new(format!("Bearer {key}")))
}

/// How the model ended, from `finish_reason`. A reason the program does not
/// know is taken as cut: the answer may be incomplete.
///
/// Called by: [`read_event`].
fn ending_of(reason: &str) -> Ending {
    match reason {
        "stop" | "tool_calls" => Ending::Complete,
        "content_filter" => Ending::Refused,
        _ => Ending::Cut,
    }
}

/// A count of tokens of the answer as a `u32`; a count that does not fit is
/// the largest one.
///
/// Called by: [`read_event`].
fn tokens(value: Option<&Value>) -> Option<u32> {
    let count = value?.as_u64()?;

    Some(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Reads one event of the answer into `reading`; gives the piece of text it
/// carried, if any. An error the service reports is an error.
///
/// Called by: `client.rs` (every event of the stream).
/// Calls: [`ending_of`], [`tokens`].
pub fn read_event(event: &SseEvent, reading: &mut Reading) -> Result<Option<String>, AiError> {
    if event.data == "[DONE]" {
        reading.done = true;
        return Ok(None);
    }

    let value: Value = serde_json::from_str(&event.data)
        .map_err(|error| AiError::UnexpectedAnswer(format!("an event that is no JSON: {error}")))?;

    if let Some(message) = value.pointer("/error/message").and_then(Value::as_str) {
        return Err(AiError::Reported(shorten(message)));
    }

    if reading.model.is_none() {
        reading.model = value.get("model").and_then(Value::as_str).map(str::to_owned);
    }

    if let Some(count) = tokens(value.pointer("/usage/prompt_tokens")) {
        reading.input_tokens = count;
    }

    if let Some(count) = tokens(value.pointer("/usage/completion_tokens")) {
        reading.output_tokens = count;
    }

    if let Some(reason) = value.pointer("/choices/0/finish_reason").and_then(Value::as_str) {
        reading.ending = Some(ending_of(reason));
    }

    match value.pointer("/choices/0/delta/content").and_then(Value::as_str) {
        Some(piece) if !piece.is_empty() => {
            reading.text.push_str(piece);
            Ok(Some(piece.to_owned()))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the protocol of OpenAI: the body, the reading of a stream.

    use super::*;

    fn data(value: &Value) -> SseEvent {
        SseEvent { name: String::new(), data: value.to_string() }
    }

    #[test]
    fn the_instructions_are_the_first_message() {
        let turns = [Turn { speaker: Speaker::User, text: "Hi".to_owned() }];
        let body = body("llama3.1", "Be brief.", &turns);

        assert_eq!(
            body["messages"],
            json!([{"role": "system", "content": "Be brief."}, {"role": "user", "content": "Hi"}])
        );
        assert_eq!(body["stream"], true);
        assert_eq!(body["model"], "llama3.1");
    }

    #[test]
    fn a_stream_is_read_into_its_text_tokens_and_ending() {
        let mut reading = Reading::default();
        let events = [
            data(&json!({"model": "llama3.1", "choices": [{"delta": {"content": "Hel"}}]})),
            data(&json!({"choices": [{"delta": {"content": "lo"}, "finish_reason": "stop"}]})),
            data(&json!({"choices": [], "usage": {"prompt_tokens": 10, "completion_tokens": 2}})),
            SseEvent { name: String::new(), data: "[DONE]".to_owned() },
        ];
        let mut pieces = Vec::new();

        for event in &events {
            pieces.extend(read_event(event, &mut reading).unwrap());
        }

        assert_eq!(pieces, ["Hel", "lo"]);
        assert_eq!(reading.text, "Hello");
        assert_eq!(reading.model.as_deref(), Some("llama3.1"));
        assert_eq!((reading.input_tokens, reading.output_tokens), (10, 2));
        assert_eq!(reading.ending, Some(Ending::Complete));
        assert!(reading.done);
    }

    #[test]
    fn a_cut_answer_and_an_error_are_told_apart() {
        let mut reading = Reading::default();

        read_event(&data(&json!({"choices": [{"delta": {}, "finish_reason": "length"}]})), &mut reading).unwrap();
        assert_eq!(reading.ending, Some(Ending::Cut));

        let error = read_event(&data(&json!({"error": {"message": "overloaded"}})), &mut reading);
        assert!(matches!(error, Err(AiError::Reported(text)) if text == "overloaded"));
    }
}
