//! Unit tests of `response.rs`, with answers in two different
//! formats, both described only by the paths of a connection.

use super::*;

use serde_json::json;

use crate::ai::connection::{AnswerMode, Connection};

/// The paths of the chat API of local servers (the template).
fn completions() -> Checked {
    Connection {
        name: "Completions".to_owned(),
        model: "m".to_owned(),
        ..Connection::template()
    }
    .check()
    .unwrap()
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
    SseEvent {
        name: String::new(),
        data: data.to_string(),
    }
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

    reader
        .event(&SseEvent {
            name: String::new(),
            data: "[DONE]".to_owned(),
        })
        .unwrap();

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

    for (reason, expected) in [("refusal", StopReason::Refused), ("max_tokens", StopReason::Cut), ("end_turn", StopReason::Complete)] {
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

    let result = reader.event(&SseEvent {
        name: String::new(),
        data: "hello".to_owned(),
    });

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
    connection.text_path = crate::ai::json_path::JsonPath::parse("choices[0].message.content").unwrap();

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
