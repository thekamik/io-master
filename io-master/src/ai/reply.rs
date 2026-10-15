//! Reading a streamed reply of Anthropic, event by event.
//!
//! A reply arrives as these events (`sse.rs` cuts the stream into
//! them):
//!
//! ```text
//! message_start         the model that answers, the input tokens
//! content_block_start   a block starts (text, thinking, ...)
//! content_block_delta   a piece of a block; "text_delta" for text
//! content_block_stop    the block is complete
//! message_delta         why the reply ended, the output tokens
//! message_stop          the reply is complete
//! ping                  nothing; keeps the connection open
//! error                 the service failed in the middle
//! ```
//!
//! Only the text of the reply is collected. Other blocks (the
//! thinking of the model, which is not shown, or the marker of a
//! fallback to another model) are passed over; so are events this
//! version does not know, which later versions of the API may add.

use serde_json::Value;

use super::ai_error::AiError;
use super::sse::SseEvent;

/// Why the model stopped writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// The reply is complete.
    EndTurn,
    /// The reply reached the largest length allowed and was cut.
    MaxTokens,
    /// The model, or the safety checks of the service, declined to
    /// answer. Part of a reply may have been written before.
    Refusal,
    /// Another reason, as the service named it.
    Other(String),
}

impl StopReason {
    /// The reason from its name in the API.
    pub fn from_name(name: &str) -> StopReason {
        match name {
            "end_turn" | "stop_sequence" => StopReason::EndTurn,
            "max_tokens" | "model_context_window_exceeded" => StopReason::MaxTokens,
            "refusal" => StopReason::Refusal,
            other => StopReason::Other(other.to_owned()),
        }
    }
}

/// What has been read of a reply so far.
#[derive(Debug, Default)]
pub struct ReplyReader {
    /// The text of the reply.
    pub text: String,
    /// The model that answered, as the service named it. It may
    /// differ from the model asked for when the service fell back
    /// to another model.
    pub model: Option<String>,
    /// Why the model stopped; `None` until it said so.
    pub stop: Option<StopReason>,
    /// Tokens of the request (the whole conversation).
    pub input_tokens: u64,
    /// Tokens of the reply.
    pub output_tokens: u64,
    /// Whether `message_stop` arrived: the reply is complete.
    pub complete: bool,
}

impl ReplyReader {
    /// Reads one event. Returns the new piece of text, if the event
    /// brought one, or the error the service reported.
    pub fn read(&mut self, event: &SseEvent) -> Result<Option<String>, AiError> {
        // "ping" carries nothing, and an event without a name is
        // not one of Anthropic's.
        if event.name == "ping" || event.name.is_empty() {
            return Ok(None);
        }

        let value: Value = serde_json::from_str(&event.data)
            .map_err(|_| AiError::UnexpectedAnswer(format!("the event {} is not JSON", event.name)))?;

        match event.name.as_str() {
            "message_start" => {
                let message = &value["message"];

                self.model = message["model"].as_str().map(str::to_owned);
                self.add_usage(&message["usage"]);
            }
            "content_block_delta" => {
                let delta = &value["delta"];

                if delta["type"] == "text_delta"
                    && let Some(text) = delta["text"].as_str()
                {
                    self.text.push_str(text);

                    return Ok(Some(text.to_owned()));
                }
            }
            "message_delta" => {
                if let Some(reason) = value["delta"]["stop_reason"].as_str() {
                    self.stop = Some(StopReason::from_name(reason));
                }

                self.add_usage(&value["usage"]);
            }
            "message_stop" => self.complete = true,
            "error" => {
                return Err(AiError::from_error_body(&value)
                    .unwrap_or_else(|| AiError::UnexpectedAnswer("an error without a description".to_owned())));
            }
            // content_block_start and _stop, and what this version
            // does not know.
            _ => {}
        }

        Ok(None)
    }

    /// Takes the token counts of a `usage` object. The service
    /// reports the input with `message_start` and the output (as
    /// a running total) with `message_delta`; the largest value
    /// seen is kept.
    fn add_usage(&mut self, usage: &Value) {
        if let Some(input) = usage["input_tokens"].as_u64() {
            self.input_tokens = self.input_tokens.max(input);
        }

        if let Some(output) = usage["output_tokens"].as_u64() {
            self.output_tokens = self.output_tokens.max(output);
        }
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "reply_tests.rs"]
mod tests;
