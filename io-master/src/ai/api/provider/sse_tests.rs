//! Unit tests of `sse.rs`.

use super::*;

fn event(name: &str, data: &str) -> SseEvent {
    SseEvent { name: name.to_owned(), data: data.to_owned() }
}

#[test]
fn whole_events_are_read() {
    let mut decoder = SseDecoder::default();

    let events = decoder.push(b"event: ping\ndata: {\"type\":\"ping\"}\n\nevent: message_stop\ndata: {}\n\n");

    assert_eq!(events, [event("ping", "{\"type\":\"ping\"}"), event("message_stop", "{}")]);
}

#[test]
fn an_event_cut_into_pieces_is_put_together() {
    let stream = "event: content_block_delta\r\ndata: {\"text\":\"Zażółć\"}\r\n\r\n".as_bytes();
    let mut decoder = SseDecoder::default();
    let mut events = Vec::new();

    // One byte at a time, also through the Polish letters, which
    // are two bytes each in UTF-8.
    for byte in stream {
        events.extend(decoder.push(std::slice::from_ref(byte)));
    }

    assert_eq!(events, [event("content_block_delta", "{\"text\":\"Zażółć\"}")]);
}

#[test]
fn an_event_is_given_out_only_when_it_ends() {
    let mut decoder = SseDecoder::default();

    assert!(decoder.push(b"event: ping\ndata: 1\n").is_empty());
    assert_eq!(decoder.push(b"\n"), [event("ping", "1")]);
}

#[test]
fn comments_unknown_fields_and_empty_lines_are_passed_over() {
    let mut decoder = SseDecoder::default();

    let events = decoder.push(b"\n\n: keep open\nid: 7\nretry: 100\ndata: x\n\n");

    assert_eq!(events, [event("", "x")]);
}

#[test]
fn several_data_lines_are_joined() {
    let mut decoder = SseDecoder::default();

    assert_eq!(decoder.push(b"data: a\ndata:b\n\n"), [event("", "a\nb")]);
}
