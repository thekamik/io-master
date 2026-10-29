//! Reading a stream of server-sent events (SSE), the form in which both
//! protocols send an answer while it is written. The stream is text in
//! lines; an empty line ends an event:
//!
//! ```text
//! event: content_block_delta
//! data: {"delta":{"type":"text_delta","text":"Hel"}}
//!
//! data: {"choices":[{"delta":{"content":"lo"}}]}
//!
//! ```
//!
//! The bytes come from the network in pieces of any size: a piece may end
//! in the middle of a line, even of a UTF-8 character. [`SseDecoder`] keeps
//! what is not complete yet and gives out only whole events. Only the
//! fields `event` and `data` are read; comments (`:`) and other fields are
//! passed over.

use crate::ai::error::AiError;

/// The longest line kept while it is not complete, in bytes: far more than
/// any event of an answer; it stops a broken service from filling the
/// memory (rule 3 of CLAUDE.md).
const MAX_LINE: usize = 1024 * 1024;

/// One event of the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The name from the `event:` line; empty when there was none.
    pub name: String,
    /// The text of the `data:` lines, joined with line breaks.
    pub data: String,
}

/// Turns pieces of the stream into whole events.
#[derive(Debug, Default)]
pub struct SseDecoder {
    /// The bytes after the last complete line.
    pending: Vec<u8>,
    /// The fields of the event being read.
    name: String,
    data: Vec<String>,
}

impl SseDecoder {
    /// Reads the next piece of the stream; gives the events it completed,
    /// in order. A line longer than [`MAX_LINE`] is an error.
    ///
    /// Called by: `client.rs` (every piece of a streamed answer).
    /// Calls: [`SseDecoder::line`].
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<SseEvent>, AiError> {
        self.pending.extend_from_slice(bytes);

        // Every complete line, without its "\n" (and the "\r" of a "\r\n");
        // what follows the last "\n" stays for the next piece.
        let mut lines = Vec::new();
        let mut start = 0;

        for (index, byte) in self.pending.iter().enumerate() {
            if *byte == b'\n' {
                let line = self.pending.get(start..index).unwrap_or_default();
                let line = line.strip_suffix(b"\r").unwrap_or(line);

                lines.push(String::from_utf8_lossy(line).into_owned());
                start = index + 1;
            }
        }

        self.pending.drain(..start);

        let mut events = Vec::new();

        for line in &lines {
            if let Some(event) = self.line(line) {
                events.push(event);
            }
        }

        ensure!(
            self.pending.len() <= MAX_LINE,
            AiError::UnexpectedAnswer("a line of the stream over 1 MiB".to_owned())
        );

        Ok(events)
    }

    /// Reads one line; gives the event an empty line ends.
    ///
    /// Called by: [`SseDecoder::push`].
    fn line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            // An empty line without fields before it ends nothing.
            if self.name.is_empty() && self.data.is_empty() {
                return None;
            }

            return Some(SseEvent {
                name: std::mem::take(&mut self.name),
                data: std::mem::take(&mut self.data).join("\n"),
            });
        }

        // "field: value"; the one space after the colon belongs to the
        // form, not to the value. A comment (":") has no field.
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);

        match field {
            "event" => value.clone_into(&mut self.name),
            "data" => self.data.push(value.to_owned()),
            _ => {}
        }

        None
    }
}

#[cfg(test)]
mod tests {
    //! Tests of the decoder: whole events, events cut into pieces, the
    //! limit of a line.

    use super::*;

    fn event(name: &str, data: &str) -> SseEvent {
        SseEvent { name: name.to_owned(), data: data.to_owned() }
    }

    #[test]
    fn whole_events_are_read() {
        let mut decoder = SseDecoder::default();
        let events =
            decoder.push(b"event: ping\ndata: {\"type\":\"ping\"}\n\nevent: message_stop\ndata: {}\n\n").unwrap();

        assert_eq!(events, [event("ping", "{\"type\":\"ping\"}"), event("message_stop", "{}")]);
    }

    #[test]
    fn an_event_cut_into_pieces_is_put_together() {
        let stream = "event: content_block_delta\r\ndata: {\"text\":\"Zażółć\"}\r\n\r\n".as_bytes();
        let mut decoder = SseDecoder::default();
        let mut events = Vec::new();

        // One byte at a time, also through the Polish letters, two bytes
        // each in UTF-8.
        for byte in stream {
            events.extend(decoder.push(std::slice::from_ref(byte)).unwrap());
        }

        assert_eq!(events, [event("content_block_delta", "{\"text\":\"Zażółć\"}")]);
    }

    #[test]
    fn comments_unknown_fields_and_empty_lines_are_passed_over() {
        let mut decoder = SseDecoder::default();

        assert_eq!(decoder.push(b"\n\n: keep open\nid: 7\ndata: a\ndata:b\n\n").unwrap(), [event("", "a\nb")]);
        assert!(decoder.push(b"data: 1\n").unwrap().is_empty());
        assert_eq!(decoder.push(b"\n").unwrap(), [event("", "1")]);
    }

    #[test]
    fn a_line_without_an_end_is_not_kept_without_a_limit() {
        let mut decoder = SseDecoder::default();

        assert!(matches!(decoder.push(&vec![b'x'; MAX_LINE + 1]), Err(AiError::UnexpectedAnswer(_))));
    }
}
