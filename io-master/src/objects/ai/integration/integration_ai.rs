//! The AI of an integration, the service that answers: the question prepared and sent,
//! the connection over TLS 1.3, the body and the stream of each protocol.

pub mod anthropic;
pub mod check;
pub mod connection;
pub mod openai;
pub mod protocol;
pub mod question;
pub mod reading;
pub mod sending;
pub mod sse;
pub mod turns;
