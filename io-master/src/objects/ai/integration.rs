//! The integration, the service that answers: the question prepared
//! and sent, the address checked, the connection over TLS 1.3, the
//! body and the stream of each protocol, chosen in one place
//! (protocol.rs).

pub mod integration_ai;
pub mod integration_model;
pub mod integration_rules;
pub mod integration_store;
