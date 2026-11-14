//! The custom format of an AI service (user, 2026-11-14): a research
//! institute builds its own AI tool, whose API takes and gives a JSON of its
//! own shape. The user writes both shapes as they are, whole, and marks with
//! placeholders the places the program needs: where its question goes, and
//! where the answer is read from. Kept in the table `ai_custom_formats` of
//! the encrypted database of the user; an integration of the protocol
//! `Protocol::Custom` names one (`Integration::custom_format`).
//!
//! ```text
//! request   {"engine": "<model>", "input": {"system": "<system>", "text": "<prompt>"}, "top_k": 40}
//!           ──► the placeholders replaced by the question ──► sent to the address of the integration
//!
//! response  {"result": {"output": "<response>", "tokens": "<output_tokens>"}, "fault": "<error>"}
//!           ──► the answer of the service laid over it ──► the text read where "<response>" stands
//! ```
//!
//! The placeholders of a request: `<model>`, `<system>`, `<prompt>` (the
//! last question), `<messages>` (the conversation, a list of roles and
//! contents), `<max_answer_tokens>`, `<temperature>`. Those of a response:
//! `<response>` (needed), `<error>`, `<model>`, `<input_tokens>`,
//! `<output_tokens>`. Everything else in either JSON is the user's own and
//! is sent, or passed over, as it is written.

/// A custom format of an AI service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomFormat {
    /// The id of the format in the database.
    pub id: i64,

    /// The name the user gave it ("The assistant of the institute").
    pub name: String,

    /// The JSON of a question as the service takes it, whole, with the
    /// placeholders of a request.
    pub request: String,

    /// The JSON of an answer as the service gives it, with the placeholders
    /// of a response; for a stream, the JSON of one event.
    pub response: String,

    /// How the answer comes.
    pub response_kind: ResponseKind,

    /// The header the API key of the integration is sent in
    /// ("Authorization"); the key goes nowhere else.
    pub key_header: String,

    /// What stands before the key in its header ("Bearer "); may be empty.
    pub key_prefix: String,
}

/// How the answer of a custom service comes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseKind {
    /// One JSON document, when the answer is whole.
    Whole,
    /// A stream of events (SSE), each of them a JSON document that carries
    /// the next piece of the answer.
    Events,
}
