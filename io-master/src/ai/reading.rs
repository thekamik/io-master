//! What an answer said so far: its text, the model that wrote it, the
//! tokens counted by the service and how the model ended. Filled event by
//! event by the reader of the protocol (`openai.rs`, `anthropic.rs`), kept
//! by `client.rs` while the answer streams.

use crate::model::ai::journal::Ending;

/// The answer read so far.
#[derive(Debug, Default)]
pub struct Reading {
    /// The text written so far.
    pub text: String,
    /// The model that answered, as the service named it.
    pub model: Option<String>,
    /// The tokens of the question and of the answer, as the service
    /// counted them (0 when it did not).
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// How the model said it ended; `None` while it writes.
    pub ending: Option<Ending>,
    /// The stream said it is over (`[DONE]`, `message_stop`).
    pub done: bool,
}
