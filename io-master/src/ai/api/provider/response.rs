//! Reading the answer of the AI service with the paths of its
//! connection (`ai/api/connection.rs`, `ai/api/provider/json_path.rs`).
//!
//! The same paths are used in both kinds of answer:
//!
//! - **a stream**: every event (`ai/api/provider/sse.rs`) is a JSON value; the
//!   text path finds the next piece of the reply in it, the other
//!   paths find the stop reason, the model, the token counts or an
//!   error when the event carries them. An event whose data is the
//!   end marker (`[DONE]`) ends the stream;
//! - **one JSON**: the whole answer is one value, read once.
//!
//! What a path does not find is simply not there: events without
//! text (the start of a stream, a "ping", the thinking of a model)
//! are passed over. Whether the reply is complete is decided at the
//! end ([`AnswerReader::complete`]).

use serde_json::Value;

use super::ai_error::{AiError, shorten};
use super::sse::SseEvent;
use crate::ai::api::connection::Checked;

/// Why the model stopped writing, as the stop reasons of the
/// connection sort the reason the service gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The reply is complete (any reason not listed as "cut" or
    /// "declined").
    Complete,
    /// The reply reached the largest length allowed and was cut.
    Cut,
    /// The model, or the safety checks of the service, declined to
    /// answer. Part of a reply may have been written before.
    Refused,
}

/// What has been read of an answer so far.
#[derive(Debug)]
pub struct AnswerReader<'a> {
    connection: &'a Checked,
    /// The text of the reply.
    pub text: String,
    /// The model that answered, as the service named it.
    pub model: Option<String>,
    /// Why the model stopped; `None` until the service said so.
    pub stop: Option<StopReason>,
    /// Tokens of the request (the whole conversation).
    pub input_tokens: u64,
    /// Tokens of the reply.
    pub output_tokens: u64,
    /// Whether the end marker of the stream arrived.
    pub done: bool,
}

impl<'a> AnswerReader<'a> {
    pub fn new(connection: &'a Checked) -> AnswerReader<'a> {
        AnswerReader {
            connection,
            text: String::new(),
            model: None,
            stop: None,
            input_tokens: 0,
            output_tokens: 0,
            done: false,
        }
    }

    /// Reads one event of a stream. Returns the new piece of text,
    /// if the event brought one, or the error the service reported.
    pub fn event(&mut self, event: &SseEvent) -> Result<Option<String>, AiError> {
        let data = event.data.trim();

        if data.is_empty() {
            return Ok(None);
        }

        if self.connection.done_marker.as_deref() == Some(data) {
            self.done = true;
            return Ok(None);
        }

        let value: Value = serde_json::from_str(data)
            .map_err(|_| AiError::UnexpectedAnswer(format!("an event of the stream is not JSON: {}", shorten(data))))?;

        self.value(&value)
    }

    /// Reads a whole answer (one JSON).
    pub fn whole(&mut self, body: &[u8]) -> Result<(), AiError> {
        let value: Value = serde_json::from_slice(body).map_err(|_| {
            AiError::UnexpectedAnswer(format!("the answer is not JSON: {}", shorten(&String::from_utf8_lossy(body))))
        })?;

        self.value(&value)?;

        if self.text.is_empty() && self.stop.is_none() {
            return Err(AiError::UnexpectedAnswer(format!(
                "the path to the text ({}) finds no text in the answer",
                self.connection.text_path.as_str()
            )));
        }

        Ok(())
    }

    /// Reads one value with every path of the connection.
    fn value(&mut self, value: &Value) -> Result<Option<String>, AiError> {
        let connection = self.connection;

        if let Some(message) = connection.error_path.as_ref().and_then(|path| path.strings(value)) {
            return Err(AiError::Reported(shorten(&message)));
        }

        if let Some(model) = connection.model_path.as_ref().and_then(|path| path.strings(value)) {
            self.model = Some(model);
        }

        if let Some(reason) = connection.stop_path.as_ref().and_then(|path| path.strings(value)) {
            self.stop = Some(if connection.cut_reasons.contains(&reason) {
                StopReason::Cut
            } else if connection.refused_reasons.contains(&reason) {
                StopReason::Refused
            } else {
                StopReason::Complete
            });
        }

        // The service may report the counts more than once (a
        // running total); the largest is kept.
        if let Some(input) = connection.input_tokens_path.as_ref().and_then(|path| path.number(value)) {
            self.input_tokens = self.input_tokens.max(input);
        }

        if let Some(output) = connection.output_tokens_path.as_ref().and_then(|path| path.number(value)) {
            self.output_tokens = self.output_tokens.max(output);
        }

        let piece = connection.text_path.strings(value).filter(|piece| !piece.is_empty());

        if let Some(piece) = &piece {
            self.text.push_str(piece);
        }

        Ok(piece)
    }

    /// Whether a stream that has ended holds a complete reply: the
    /// end marker came, or a stop reason. A connection that has
    /// neither a stop path nor an end marker cannot tell, so the end
    /// of the stream itself counts.
    pub fn complete(&self) -> bool {
        self.done
            || self.stop.is_some()
            || (self.connection.stop_path.is_none() && self.connection.done_marker.is_none())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `response.rs`, with answers in two different
    //! formats, both described only by the paths of a connection.

    use super::*;

    use serde_json::json;

    use crate::model::ai::{AnswerMode, Connection};

    /// The paths of the chat API of local servers (the template).
    fn completions() -> Checked {
        Connection { name: "Completions".to_owned(), model: "m".to_owned(), ..Connection::template() }.check().unwrap()
    }

    /// The paths of a format with named events and blocks of content.
    fn messages_format() -> Checked {
        Connection {
            name: "Messages".to_owned(),
            model: "m".to_owned(),
            text_path: "delta.text".to_owned(),
            stop_path: "delta.stop_reason".to_owned(),
            cut_reasons: "max_tokens".to_owned(),
            refused_reasons: "refusal".to_owned(),
            error_path: "error.message".to_owned(),
            model_path: "message.model".to_owned(),
            input_tokens_path: "message.usage.input_tokens".to_owned(),
            output_tokens_path: "usage.output_tokens".to_owned(),
            done_marker: String::new(),
            ..Connection::template()
        }
        .check()
        .unwrap()
    }

    fn event(data: &serde_json::Value) -> SseEvent {
        SseEvent { name: String::new(), data: data.to_string() }
    }

    #[test]
    fn a_completions_stream_is_read() {
        let connection = completions();
        let mut reader = AnswerReader::new(&connection);

        let first = json!({"model": "llama", "choices": [{"delta": {"role": "assistant", "content": "Hel"}}]});
        let second = json!({"choices": [{"delta": {"content": "lo"}, "finish_reason": null}]});
        let last = json!({"choices": [{"delta": {}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 7, "completion_tokens": 2}});

        assert_eq!(reader.event(&event(&first)).unwrap().as_deref(), Some("Hel"));
        assert_eq!(reader.event(&event(&second)).unwrap().as_deref(), Some("lo"));
        assert_eq!(reader.event(&event(&last)).unwrap(), None);
        assert!(!reader.done);

        reader.event(&SseEvent { name: String::new(), data: "[DONE]".to_owned() }).unwrap();

        assert!(reader.done && reader.complete());
        assert_eq!(reader.text, "Hello");
        assert_eq!(reader.model.as_deref(), Some("llama"));
        assert_eq!(reader.stop, Some(StopReason::Complete));
        assert_eq!((reader.input_tokens, reader.output_tokens), (7, 2));
    }

    #[test]
    fn a_stream_of_another_format_is_read_with_its_paths() {
        let connection = messages_format();
        let mut reader = AnswerReader::new(&connection);

        let events = [
            json!({"type": "message_start", "message": {"model": "big-model", "usage": {"input_tokens": 10, "output_tokens": 1}}}),
            json!({"type": "ping"}),
            json!({"type": "content_block_delta", "delta": {"type": "thinking_delta", "thinking": "hidden"}}),
            json!({"type": "content_block_delta", "delta": {"type": "text_delta", "text": "Hi"}}),
            json!({"type": "message_delta", "delta": {"stop_reason": "max_tokens"}, "usage": {"output_tokens": 5}}),
            json!({"type": "message_stop"}),
        ];

        for value in &events {
            reader.event(&event(value)).unwrap();
        }

        assert_eq!(reader.text, "Hi");
        assert_eq!(reader.model.as_deref(), Some("big-model"));
        assert_eq!(reader.stop, Some(StopReason::Cut));
        assert_eq!((reader.input_tokens, reader.output_tokens), (10, 5));
        assert!(reader.complete());
    }

    #[test]
    fn stop_reasons_are_sorted_by_the_connection() {
        let connection = messages_format();

        for (reason, expected) in
            [("refusal", StopReason::Refused), ("max_tokens", StopReason::Cut), ("end_turn", StopReason::Complete)]
        {
            let mut reader = AnswerReader::new(&connection);
            reader.event(&event(&json!({"delta": {"stop_reason": reason}}))).unwrap();

            assert_eq!(reader.stop, Some(expected), "{reason}");
        }
    }

    #[test]
    fn an_error_in_the_stream_is_reported() {
        let connection = messages_format();
        let mut reader = AnswerReader::new(&connection);

        let error = reader.event(&event(&json!({"type": "error", "error": {"message": "Overloaded"}})));

        assert_eq!(error, Err(AiError::Reported("Overloaded".to_owned())));
    }

    #[test]
    fn an_event_that_is_not_json_is_an_error() {
        let connection = completions();
        let mut reader = AnswerReader::new(&connection);

        let result = reader.event(&SseEvent { name: String::new(), data: "hello".to_owned() });

        assert!(matches!(result, Err(AiError::UnexpectedAnswer(_))));
    }

    #[test]
    fn a_stream_without_its_end_is_not_complete() {
        let connection = completions();
        let mut reader = AnswerReader::new(&connection);

        reader.event(&event(&json!({"choices": [{"delta": {"content": "Part"}}]}))).unwrap();

        assert!(!reader.complete());

        // A connection that cannot tell takes the end of the stream.
        let mut silent = completions();
        silent.stop_path = None;
        silent.done_marker = None;

        assert!(AnswerReader::new(&silent).complete());
    }

    #[test]
    fn a_whole_answer_is_read() {
        let mut connection = completions();
        connection.answer = AnswerMode::Whole;
        connection.text_path =
            crate::ai::api::provider::json_path::JsonPath::parse("choices[0].message.content").unwrap();

        let mut reader = AnswerReader::new(&connection);
        let body = json!({"model": "llama", "choices": [{"message": {"content": "Whole"}, "finish_reason": "length"}]});

        reader.whole(body.to_string().as_bytes()).unwrap();

        assert_eq!(reader.text, "Whole");
        assert_eq!(reader.stop, Some(StopReason::Cut));

        // A path that finds nothing is explained.
        let mut reader = AnswerReader::new(&connection);
        let error = reader.whole(br#"{"something": "else"}"#).unwrap_err();
        assert!(matches!(error, AiError::UnexpectedAnswer(text) if text.contains("choices[0].message.content")));

        assert!(matches!(AnswerReader::new(&connection).whole(b"<html>"), Err(AiError::UnexpectedAnswer(_))));
    }
}
