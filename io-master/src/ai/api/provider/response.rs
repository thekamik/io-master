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
use crate::ai::api::connection::Checked;
use super::sse::SseEvent;

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

        let value: Value = serde_json::from_str(data).map_err(|_| {
            AiError::UnexpectedAnswer(format!("an event of the stream is not JSON: {}", shorten(data)))
        })?;

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

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
