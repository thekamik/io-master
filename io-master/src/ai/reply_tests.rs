//! Unit tests of `reply.rs`.

use super::*;

fn event(name: &str, data: &str) -> SseEvent {
    SseEvent {
        name: name.to_owned(),
        data: data.to_owned(),
    }
}

/// The events of a short reply, as Anthropic sends them.
fn reply_events() -> Vec<SseEvent> {
    vec![
        event(
            "message_start",
            r#"{"type":"message_start","message":{"id":"msg_1","type":"message","model":"claude-opus-5-5","usage":{"input_tokens":25,"output_tokens":1}}}"#,
        ),
        event("content_block_start", r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#),
        event("content_block_delta", r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"abc"}}"#),
        event("content_block_stop", r#"{"type":"content_block_stop","index":0}"#),
        event("ping", r#"{"type":"ping"}"#),
        event("content_block_start", r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#),
        event("content_block_delta", r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Hello"}}"#),
        event("content_block_delta", r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":", evaluator."}}"#),
        event("content_block_stop", r#"{"type":"content_block_stop","index":1}"#),
        event("message_delta", r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":12}}"#),
        event("message_stop", r#"{"type":"message_stop"}"#),
    ]
}

#[test]
fn a_reply_is_read_with_its_pieces_and_counts() {
    let mut reader = ReplyReader::default();
    let mut pieces = Vec::new();

    for event in reply_events() {
        if let Some(piece) = reader.read(&event).unwrap() {
            pieces.push(piece);
        }
    }

    assert_eq!(pieces, ["Hello", ", evaluator."]);
    assert_eq!(reader.text, "Hello, evaluator.");
    assert_eq!(reader.model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(reader.stop, Some(StopReason::EndTurn));
    assert_eq!((reader.input_tokens, reader.output_tokens), (25, 12));
    assert!(reader.complete);
}

#[test]
fn an_error_event_becomes_an_error() {
    let mut reader = ReplyReader::default();

    let error = reader
        .read(&event("error", r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#))
        .unwrap_err();

    assert_eq!(error, AiError::Overloaded);
}

#[test]
fn a_refusal_is_told() {
    let mut reader = ReplyReader::default();

    reader
        .read(&event("message_delta", r#"{"type":"message_delta","delta":{"stop_reason":"refusal","stop_details":{"category":"cyber"}},"usage":{"output_tokens":0}}"#))
        .unwrap();

    assert_eq!(reader.stop, Some(StopReason::Refusal));
}

#[test]
fn unknown_events_are_passed_over_and_broken_ones_are_errors() {
    let mut reader = ReplyReader::default();

    assert_eq!(reader.read(&event("future_event", r#"{"type":"future_event"}"#)).unwrap(), None);
    assert!(matches!(
        reader.read(&event("content_block_delta", "not json")),
        Err(AiError::UnexpectedAnswer(_))
    ));
}

#[test]
fn stop_reasons_are_named() {
    assert_eq!(StopReason::from_name("end_turn"), StopReason::EndTurn);
    assert_eq!(StopReason::from_name("max_tokens"), StopReason::MaxTokens);
    assert_eq!(StopReason::from_name("refusal"), StopReason::Refusal);
    assert_eq!(StopReason::from_name("pause_turn"), StopReason::Other("pause_turn".to_owned()));
}
