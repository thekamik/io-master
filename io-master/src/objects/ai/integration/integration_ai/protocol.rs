//! The one place that chooses the protocol of an integration (user,
//! 2026-11-01): everything that differs between the protocols is asked of
//! the file of the protocol here, so that a protocol added is joined in this
//! file alone.
//!
//! ```text
//!                     Protocol::OpenAiChat    Protocol::AnthropicMessages   Protocol::Custom (by its format)
//! needs_key           no (a local server)     yes                           no (the tool of an institute)
//! body          ──►   openai.rs: body         anthropic.rs: body            custom_format_ai.rs: custom_body
//! key_headers   ──►   openai.rs: key_header   anthropic.rs: key_headers     the header of the format
//! read_event    ──►   openai.rs: read_event   anthropic.rs: read_event      custom_format_ai.rs: read_custom_event
//! ```
//!
//! A custom service that answers with one JSON document has no events: its
//! answer is read by `connection.rs`: `read_whole_answer`.

use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::objects::ai::custom_format::custom_format_ai::{CustomExchange, custom_body, read_custom_event};
use crate::objects::ai::custom_format::custom_format_model::CustomFormat;
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
/// completions of `OpenAI` and for a custom service.
///
/// Called by: `objects/ai/integration/integration_ai/question.rs`: `prepare`; [`key_headers`].
pub fn needs_key(protocol: Protocol) -> bool {
    match protocol {
        Protocol::OpenAiChat | Protocol::Custom => false,
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
/// - `temperature`: the temperature of the agent in hundredths (`Some(20)`
///   is written 0.2); `None` writes none.
/// - `format`: the custom format of the integration, for `Protocol::Custom`.
///
/// Output: the body as JSON (`{"model": "llama3.1", "stream": true, ...}`).
///
/// Called by: `objects/ai/integration/integration_ai/question.rs`: `prepare`.
/// Calls: `objects/ai/integration/integration_ai/openai.rs`: `body`;
/// `objects/ai/integration/integration_ai/anthropic.rs`: `body`;
/// `objects/ai/custom_format/custom_format_ai.rs`: `custom_body`; [`no_format`].
pub fn body(
    protocol: Protocol,
    model: &str,
    system: &str,
    turns: &[Turn],
    answer_tokens: u32,
    temperature: Option<u32>,
    format: Option<&CustomFormat>,
) -> Result<Value, AiError> {
    let mut body = match protocol {
        Protocol::OpenAiChat => openai::body(model, system, turns),
        Protocol::AnthropicMessages => anthropic::body(model, system, turns, answer_tokens),
        // The request of the format says itself where a temperature goes.
        Protocol::Custom => {
            return custom_body(format.ok_or_else(no_format)?, model, system, turns, answer_tokens, temperature);
        }
    };

    // Both protocols name it "temperature", a number; none is asked for
    // when the agent gives none (a model that refuses the parameter). The
    // body of either protocol is an object.
    if let (Some(hundredths), Some(fields)) = (temperature, body.as_object_mut()) {
        fields.insert("temperature".to_owned(), json!(f64::from(hundredths) / 100.0));
    }

    Ok(body)
}

/// The error of a custom protocol spoken without its format: an integration
/// the rules never store.
///
/// Called by: [`body`], [`key_headers`], [`read_event`].
fn no_format() -> AiError {
    AiError::Refused("an integration of the custom protocol without its custom format".to_owned())
}

/// The headers that carry the key `key` in `protocol`.
///
/// Input: `protocol`, the protocol of the integration; `key`, the API key,
/// `None` when the integration has none; `custom`, how a custom service is
/// spoken with, given for `Protocol::Custom`.
/// Output: the headers (`[("authorization", "Bearer ...")]`), none for a
/// service without a key; `AiError::NoKey` when the protocol needs a key and
/// there is none, `AiError::Refused` for the custom protocol without its
/// exchange.
///
/// Called by: `objects/ai/integration/integration_ai/connection.rs`: `post`.
/// Calls: [`needs_key`]; `objects/ai/integration/integration_ai/openai.rs`: `key_header`;
/// `objects/ai/integration/integration_ai/anthropic.rs`: `key_headers`; [`no_format`].
pub fn key_headers(
    protocol: Protocol,
    key: Option<&str>,
    custom: Option<&CustomExchange>,
) -> Result<Vec<(String, Zeroizing<String>)>, AiError> {
    let mut headers = Vec::with_capacity(MAX_KEY_HEADERS);
    let Some(key) = key else {
        ensure!(!needs_key(protocol), AiError::NoKey);
        return Ok(headers);
    };

    match protocol {
        Protocol::OpenAiChat => {
            let (name, value) = openai::key_header(key);

            headers.push((name.to_owned(), value));
        }
        Protocol::AnthropicMessages => {
            for (name, value) in anthropic::key_headers(key) {
                headers.push((name.to_owned(), value));
            }
        }
        // The format names the header of the key and what stands before it.
        Protocol::Custom => {
            let custom = custom.ok_or_else(no_format)?;

            headers.push((custom.key_header.clone(), Zeroizing::new(format!("{}{key}", custom.key_prefix))));
        }
    }

    Ok(headers)
}

/// Reads one event of an answer in `protocol` into `reading`.
///
/// Input: `protocol`, the protocol of the integration; `event`, one event of
/// the stream; `reading`, what was read of the answer so far; `custom`, how
/// a custom service is spoken with, given for `Protocol::Custom`.
/// Output: the piece of text the event carried (`Some("Hel")`), if any; an
/// error for an event that cannot be read or an error the service reports.
///
/// Called by: `objects/ai/integration/integration_ai/connection.rs`: `read_stream`.
/// Calls: `objects/ai/integration/integration_ai/openai.rs`: `read_event`;
/// `objects/ai/integration/integration_ai/anthropic.rs`: `read_event`;
/// `objects/ai/custom_format/custom_format_ai.rs`: `read_custom_event`; [`no_format`].
pub fn read_event(
    protocol: Protocol,
    event: &SseEvent,
    reading: &mut Reading,
    custom: Option<&CustomExchange>,
) -> Result<Option<String>, AiError> {
    match protocol {
        Protocol::OpenAiChat => openai::read_event(event, reading),
        Protocol::AnthropicMessages => anthropic::read_event(event, reading),
        Protocol::Custom => read_custom_event(event, custom.ok_or_else(no_format)?, reading),
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the choice of a protocol: the key each one needs, the
    //! temperature written into the body of either.

    use super::*;

    use crate::objects::ai::custom_format::custom_format_ai::custom_exchange;
    use crate::test_support::database::custom_format;

    #[test]
    fn only_anthropic_needs_a_key_and_each_protocol_has_its_headers() {
        assert!(key_headers(Protocol::OpenAiChat, None, None).unwrap().is_empty());
        assert!(matches!(key_headers(Protocol::AnthropicMessages, None, None), Err(AiError::NoKey)));

        let names = |protocol| -> Vec<String> {
            key_headers(protocol, Some("k"), None).unwrap().into_iter().map(|(name, _)| name).collect()
        };

        assert_eq!(names(Protocol::OpenAiChat), ["authorization"]);
        assert_eq!(names(Protocol::AnthropicMessages), ["x-api-key", "anthropic-version"]);
    }

    #[test]
    fn a_custom_service_gets_its_key_in_the_header_of_its_format() {
        let format =
            CustomFormat { key_header: "X-Api-Key".to_owned(), key_prefix: "Token ".to_owned(), ..custom_format() };
        let exchange = custom_exchange(&format).unwrap();
        let headers = key_headers(Protocol::Custom, Some("k"), Some(&exchange)).unwrap();

        assert_eq!(headers.len(), 1);
        assert_eq!((headers[0].0.as_str(), headers[0].1.as_str()), ("X-Api-Key", "Token k"));
        // A custom service may need no key; without its format it is refused.
        assert!(key_headers(Protocol::Custom, None, Some(&exchange)).unwrap().is_empty());
        assert!(matches!(key_headers(Protocol::Custom, Some("k"), None), Err(AiError::Refused(_))));
        assert!(matches!(body(Protocol::Custom, "m", "", &[], 100, None, None), Err(AiError::Refused(_))));
    }

    #[test]
    fn a_temperature_is_written_only_when_the_agent_gives_one() {
        for protocol in [Protocol::OpenAiChat, Protocol::AnthropicMessages] {
            let without = body(protocol, "m", "", &[], 100, None, None).unwrap();
            let with = body(protocol, "m", "", &[], 100, Some(20), None).unwrap();
            let zero = body(protocol, "m", "", &[], 100, Some(0), None).unwrap();

            assert!(without.get("temperature").is_none());
            assert_eq!(with["temperature"].to_string(), "0.2");
            assert_eq!(zero["temperature"].to_string(), "0.0");
        }
    }
}
