//! The messages of Anthropic (`/v1/messages`): the body of a question, its
//! headers and the reading of the answer while it streams.
//!
//! ```text
//! the question   {"model", "max_tokens", "stream": true, "system", "messages": [{"role": "user"}, ...]}
//!                headers  x-api-key, anthropic-version: 2026-10-21
//! the answer     event: message_start         {"message": {"model", "usage": {"input_tokens"}}}
//!                event: content_block_delta   {"delta": {"type": "text_delta", "text": "Hel"}}
//!                event: message_delta         {"delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens"}}
//!                event: message_stop
//!                event: error                 {"error": {"type", "message"}}
//! ```
//!
//! Only the text of the answer is read: the blocks of the thinking of a
//! model (`thinking_delta`) are passed over. The service always needs a key.

use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::ai::conversation::Turn;
use crate::ai::error::{AiError, shorten};
use crate::ai::reading::Reading;
use crate::ai::sse::SseEvent;
use crate::model::ai::chat::Speaker;
use crate::model::ai::journal::Ending;

/// The version of the API the program speaks (the header
/// `anthropic-version`).
const VERSION: &str = "2026-10-21";

/// The body of a question: the instructions in the field `system`, the
/// turns in `messages`, the answer at most `answer_tokens` long.
///
/// Called by: `ask.rs` ([`crate::ai::ask::prepare`]).
pub fn body(model: &str, system: &str, turns: &[Turn], answer_tokens: u32) -> Value {
    let mut messages = Vec::with_capacity(turns.len());

    for turn in turns {
        let role = match turn.speaker {
            Speaker::User => "user",
            Speaker::Ai => "assistant",
        };

        messages.push(json!({"role": role, "content": turn.text}));
    }

    if system.trim().is_empty() {
        return json!({"model": model, "max_tokens": answer_tokens, "stream": true, "messages": messages});
    }

    json!({"model": model, "max_tokens": answer_tokens, "stream": true, "system": system, "messages": messages})
}

/// The headers of the key and of the version of the API.
///
/// Called by: `client.rs` (the request).
pub fn key_headers(key: &str) -> [(&'static str, Zeroizing<String>); 2] {
    [("x-api-key", Zeroizing::new(key.to_owned())), ("anthropic-version", Zeroizing::new(VERSION.to_owned()))]
}

/// How the model ended, from `stop_reason`. A reason the program does not
/// know (or `max_tokens`) is taken as cut: the answer may be incomplete.
///
/// Called by: [`read_event`].
fn ending_of(reason: &str) -> Ending {
    match reason {
        "end_turn" | "stop_sequence" => Ending::Complete,
        "refusal" => Ending::Refused,
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
    let value: Value = serde_json::from_str(&event.data)
        .map_err(|error| AiError::UnexpectedAnswer(format!("an event that is no JSON: {error}")))?;
    let kind = value.get("type").and_then(Value::as_str).unwrap_or(event.name.as_str());

    match kind {
        "message_start" => {
            reading.model = value.pointer("/message/model").and_then(Value::as_str).map(str::to_owned);
            reading.input_tokens = tokens(value.pointer("/message/usage/input_tokens")).unwrap_or(0);
        }
        "message_delta" => {
            if let Some(reason) = value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                reading.ending = Some(ending_of(reason));
            }
            if let Some(count) = tokens(value.pointer("/usage/output_tokens")) {
                reading.output_tokens = count;
            }
        }
        "content_block_delta" if value.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") => {
            let piece = value.pointer("/delta/text").and_then(Value::as_str).unwrap_or_default();

            reading.text.push_str(piece);
            return Ok(Some(piece.to_owned()));
        }
        "message_stop" => reading.done = true,
        "error" => {
            let message = value.pointer("/error/message").and_then(Value::as_str).unwrap_or("no explanation");

            return Err(AiError::Reported(shorten(message)));
        }
        _ => {}
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    //! Tests of the protocol of Anthropic: the body, the reading of a
    //! stream.

    use super::*;

    fn sse(name: &str, value: &Value) -> SseEvent {
        SseEvent { name: name.to_owned(), data: value.to_string() }
    }

    #[test]
    fn the_instructions_go_into_their_own_field() {
        let turns = [Turn { speaker: Speaker::User, text: "Hi".to_owned() }];

        assert_eq!(
            body("claude-opus-5-5", "Be brief.", &turns, 4096),
            json!({
                "model": "claude-opus-5-5",
                "max_tokens": 4096,
                "stream": true,
                "messages": [{"role": "user", "content": "Hi"}],
                "system": "Be brief."
            })
        );
        assert!(body("m", " ", &turns, 1).get("system").is_none());
    }

    #[test]
    fn a_stream_is_read_into_its_text_tokens_and_ending() {
        let mut reading = Reading::default();
        let events = [
            sse(
                "message_start",
                &json!({"type": "message_start", "message": {"model": "claude-opus-5-5", "usage": {"input_tokens": 12}}}),
            ),
            sse(
                "content_block_delta",
                &json!({"type": "content_block_delta", "delta": {"type": "thinking_delta", "thinking": ""}}),
            ),
            sse(
                "content_block_delta",
                &json!({"type": "content_block_delta", "delta": {"type": "text_delta", "text": "Hi "}}),
            ),
            sse(
                "content_block_delta",
                &json!({"type": "content_block_delta", "delta": {"type": "text_delta", "text": "there"}}),
            ),
            sse("ping", &json!({"type": "ping"})),
            sse(
                "message_delta",
                &json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 3}}),
            ),
            sse("message_stop", &json!({"type": "message_stop"})),
        ];
        let mut pieces = Vec::new();

        for event in &events {
            pieces.extend(read_event(event, &mut reading).unwrap());
        }

        assert_eq!(pieces, ["Hi ", "there"]);
        assert_eq!(reading.model.as_deref(), Some("claude-opus-5-5"));
        assert_eq!((reading.input_tokens, reading.output_tokens), (12, 3));
        assert_eq!(reading.ending, Some(Ending::Complete));
        assert!(reading.done);
    }

    #[test]
    fn a_refusal_a_cut_and_an_error_are_told_apart() {
        let mut reading = Reading::default();
        let delta =
            |reason: &str| sse("message_delta", &json!({"type": "message_delta", "delta": {"stop_reason": reason}}));

        read_event(&delta("refusal"), &mut reading).unwrap();
        assert_eq!(reading.ending, Some(Ending::Refused));

        read_event(&delta("max_tokens"), &mut reading).unwrap();
        assert_eq!(reading.ending, Some(Ending::Cut));

        let error =
            sse("error", &json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}));
        assert!(matches!(read_event(&error, &mut reading), Err(AiError::Reported(text)) if text == "Overloaded"));
    }
}
