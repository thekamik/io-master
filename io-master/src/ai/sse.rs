//! Reading a stream of server-sent events (SSE), the format in
//! which Anthropic sends a reply while it is written.
//!
//! The stream is text in lines; an empty line ends an event:
//!
//! ```text
//! event: content_block_delta
//! data: {"type":"content_block_delta","delta":{"text":"Hello"}}
//!
//! event: ping
//! data: {"type":"ping"}
//!
//! ```
//!
//! The bytes arrive from the network in pieces of any size: a piece
//! may end in the middle of a line, even in the middle of a UTF-8
//! character. [`SseDecoder`] keeps what is not complete yet and
//! gives out only whole events. Only what Anthropic uses is read:
//! the fields `event` and `data`; comments (lines starting with
//! `:`) and other fields are passed over.

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
    /// Bytes after the last complete line.
    pending: Vec<u8>,
    /// The fields of the event being read.
    name: String,
    data: Vec<String>,
}

impl SseDecoder {
    /// Reads the next piece of the stream; returns the events it
    /// completed, in order.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.pending.extend_from_slice(bytes);

        let mut events = Vec::new();

        // Every complete line; the rest stays for the next piece.
        while let Some(end) = self.pending.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();

            // Without "\n", and without "\r" of a "\r\n" line end.
            let line = line.strip_suffix(b"\n").unwrap_or(&line);
            let line = line.strip_suffix(b"\r").unwrap_or(line);

            if let Some(event) = self.line(&String::from_utf8_lossy(line)) {
                events.push(event);
            }
        }

        events
    }

    /// Reads one line; returns the event that an empty line ends.
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

        // A comment, used by some servers to keep the line open.
        if line.starts_with(':') {
            return None;
        }

        // "field: value"; one space after the colon belongs to the
        // format, not to the value.
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);

        match field {
            "event" => self.name = value.to_owned(),
            "data" => self.data.push(value.to_owned()),
            _ => {}
        }

        None
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "sse_tests.rs"]
mod tests;
