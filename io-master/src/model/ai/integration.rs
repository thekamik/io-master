//! An integration with an external AI service: where the program sends
//! its questions and how it speaks with the service. Kept in the table
//! `ai_integrations` of the encrypted database of the user; its API key
//! is kept apart (`api_key.rs`).

/// An integration with an AI service (user, 2026-10-28).
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub struct Integration {
    /// The id of the integration in the database.
    pub id: i64,

    /// The name the user gave it ("Ollama on this computer").
    pub name: String,

    /// The address of the service ("http://127.0.0.1:11434/v1/chat/completions").
    pub url: String,

    /// The name of the model at the service ("llama3.1:70b").
    pub model: String,

    /// The way the program speaks with the service.
    pub protocol: Protocol,

    /// The CA certificate (PEM) that alone is trusted for the service;
    /// `None`: the trusted certificates of the system.
    pub ca_certificate: Option<String>,

    /// How many tokens the model takes in one question (its context). A
    /// question that does not fit is refused before it is sent, never
    /// shortened in silence.
    pub context_size: u32,
}

/// The way the program speaks with an AI service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "a new model (2026-10-28), not used by the code yet")]
pub enum Protocol {
    /// The chat completions of OpenAI (`/v1/chat/completions`), which
    /// most local servers offer as well: Ollama, llama.cpp, vLLM.
    OpenAiChat,
    /// The messages of Anthropic (`/v1/messages`).
    AnthropicMessages,
}
