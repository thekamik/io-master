//! The communication with the provider: HTTP, TLS, request, answer, stream.
//! Reachable only from `ai/api/`.

pub mod ai_error;
pub mod client;
pub mod json_path;
pub mod request;
pub mod response;
pub mod sse;
