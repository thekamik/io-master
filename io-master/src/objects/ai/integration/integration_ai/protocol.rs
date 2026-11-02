//! The one place that chooses the protocol of an integration (user,
//! 2026-11-01): everything that differs between the protocols is asked of
//! the file of the protocol here, so that a protocol added is joined in this
//! file alone.
//!
//! ```text
//!                     Protocol::OpenAiChat    Protocol::AnthropicMessages
//! needs_key           no (a local server)     yes
//! body          ──►   openai.rs: body         anthropic.rs: body
//! key_headers   ──►   openai.rs: key_header   anthropic.rs: key_headers
//! read_event    ──►   openai.rs: read_event   anthropic.rs: read_event
//! ```

use serde_json::Value;
use zeroize::Zeroizing;

use crate::objects::ai::integration::integration_ai::reading::Reading;
use crate::objects::ai::integration::integration_ai::sse::SseEvent;
use crate::objects::ai::integration::integration_ai::turns::Turn;
use crate::objects::ai::integration::integration_ai::{anthropic, openai};
use crate::objects::ai::integration::integration_model::Protocol;
use crate::objects::ai_error::AiError;

/// The most headers of a key: two, of Anthropic (the key and the version).
const MAX_KEY_HEADERS: usize = 2;

/// Whether the service of `protocol` always needs an API key: a local
/// server of the chat completions of `OpenAI` needs none.
///
/// Input: `protocol`, the protocol of an integration.
/// Output: `true` for the messages of Anthropic, `false` for the chat
/// completions of `OpenAI`.
///
/// Called by: `objects/ai/integration/integration_ai/question.rs`: `prepare`; [`key_headers`].
pub fn needs_key(protocol: Protocol) -> bool {
    match protocol {
        Protocol::OpenAiChat => false,
        Protocol::AnthropicMessages => true,
    }
}

/// The body of a question in `protocol`.
///
/// Input:
/// - `protocol`: the protocol of the integration.
/// - `model`: the model asked ("llama3.1").
/// - `system`: the instructions; empty for none.
/// - `turns`: the conversation, starting with the user (`turns.rs`).
/// - `answer_tokens`: the longest answer, at least 1; the chat completions
///   of `OpenAI` do not take it.
///
/// Output: the body as JSON (`{"model": "llama3.1", "stream": true, ...}`).
///
/// Called by: `objects/ai/integration/integration_ai/question.rs`: `prepare`.
/// Calls: `objects/ai/integration/integration_ai/openai.rs`: `body`;
/// `objects/ai/integration/integration_ai/anthropic.rs`: `body`.
pub fn body(protocol: Protocol, model: &str, system: &str, turns: &[Turn], answer_tokens: u32) -> Value {
    match protocol {
        Protocol::OpenAiChat => openai::body(model, system, turns),
        Protocol::AnthropicMessages => anthropic::body(model, system, turns, answer_tokens),
    }
}

/// The headers that carry the key `key` in `protocol`.
///
/// Input: `protocol`, the protocol of the integration; `key`, the API key,
/// `None` when the integration has none.
/// Output: the headers (`[("authorization", "Bearer ...")]`), none for a
/// service without a key; `AiError::NoKey` when the protocol needs a key and
/// there is none.
///
/// Called by: `objects/ai/integration/integration_ai/connection.rs`: `post`.
/// Calls: [`needs_key`]; `objects/ai/integration/integration_ai/openai.rs`: `key_header`;
/// `objects/ai/integration/integration_ai/anthropic.rs`: `key_headers`.
pub fn key_headers(protocol: Protocol, key: Option<&str>) -> Result<Vec<(&'static str, Zeroizing<String>)>, AiError> {
    let mut headers = Vec::with_capacity(MAX_KEY_HEADERS);
    let Some(key) = key else {
        ensure!(!needs_key(protocol), AiError::NoKey);
        return Ok(headers);
    };

    match protocol {
        Protocol::OpenAiChat => headers.push(openai::key_header(key)),
        Protocol::AnthropicMessages => headers.extend(anthropic::key_headers(key)),
    }

    Ok(headers)
}

/// Reads one event of an answer in `protocol` into `reading`.
///
/// Input: `protocol`, the protocol of the integration; `event`, one event of
/// the stream; `reading`, what was read of the answer so far.
/// Output: the piece of text the event carried (`Some("Hel")`), if any; an
/// error for an event that cannot be read or an error the service reports.
///
/// Called by: `objects/ai/integration/integration_ai/connection.rs`: `read_stream`.
/// Calls: `objects/ai/integration/integration_ai/openai.rs`: `read_event`;
/// `objects/ai/integration/integration_ai/anthropic.rs`: `read_event`.
pub fn read_event(protocol: Protocol, event: &SseEvent, reading: &mut Reading) -> Result<Option<String>, AiError> {
    match protocol {
        Protocol::OpenAiChat => openai::read_event(event, reading),
        Protocol::AnthropicMessages => anthropic::read_event(event, reading),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the choice of a protocol: the key each one needs.

    use super::*;

    #[test]
    fn only_anthropic_needs_a_key_and_each_protocol_has_its_headers() {
        assert!(key_headers(Protocol::OpenAiChat, None).unwrap().is_empty());
        assert!(matches!(key_headers(Protocol::AnthropicMessages, None), Err(AiError::NoKey)));

        let openai: Vec<&str> =
            key_headers(Protocol::OpenAiChat, Some("k")).unwrap().iter().map(|(name, _)| *name).collect();
        let anthropic: Vec<&str> =
            key_headers(Protocol::AnthropicMessages, Some("k")).unwrap().iter().map(|(name, _)| *name).collect();

        assert_eq!(openai, ["authorization"]);
        assert_eq!(anthropic, ["x-api-key", "anthropic-version"]);
    }
}
