//! An integration with an external AI service: where the program sends
//! its questions and how it speaks with the service. Kept in the table
//! `ai_integrations` of the encrypted database of the user; its API key
//! is kept apart (`objects/ai/api_key/api_key_model.rs`).

/// An integration with an AI service (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Integration {
    /// The id of the integration in the database.
    pub id: i64,

    /// The name the user gave it ("Ollama on this computer").
    pub name: String,

    /// The address of the service (`"http://127.0.0.1:11434/v1/chat/completions"`).
    pub url: String,

    /// The name of the model at the service ("llama3.1:70b").
    pub model: String,

    /// The way the program speaks with the service.
    pub protocol: Protocol,

    /// The custom format the service is spoken with (`CustomFormat` of
    /// `objects/ai/custom_format/custom_format_model.rs`): given for the
    /// protocol `Protocol::Custom`, and only for it (user, 2026-11-14).
    pub custom_format: Option<i64>,

    /// The CA certificate (PEM) that alone is trusted for the service;
    /// `None`: the trusted certificates of the system.
    pub ca_certificate: Option<String>,

    /// How many tokens the model takes in one question (its context). A
    /// question that does not fit is refused before it is sent, never
    /// shortened in silence.
    pub context_size: u32,

    /// How many characters of a text the model reads as one token, on
    /// average (user, 2026-11-02). The tokens are counted by the service, each
    /// model by its own tokenizer; this is only the estimate by which the
    /// program measures a question against `context_size` before it is sent,
    /// a guard against a local server that shortens a question over its
    /// context in silence. Fewer characters than the model uses err on the
    /// safe side: a question is refused rather than cut.
    pub characters_per_token: u32,

    /// How long, in seconds, a connection to the service may take to open.
    pub connect_timeout_seconds: u32,

    /// How long, in seconds, the stream of an answer may stay silent: a
    /// local model on a slow computer may think long before its first piece;
    /// a longer silence is taken for a broken connection.
    pub stream_silence_seconds: u32,
}

/// The way the program speaks with an AI service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// The chat completions of `OpenAI` (`/v1/chat/completions`), which
    /// most local servers offer as well: Ollama, llama.cpp, vLLM.
    OpenAiChat,
    /// The messages of Anthropic (`/v1/messages`).
    AnthropicMessages,
    /// A JSON of the service's own, as the custom format of the integration
    /// writes it: the tool of a research institute.
    Custom,
}
