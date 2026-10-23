//! The element "connection line" of the view Chat: which AI connection
//! answers in the chat (page: `ui/src/views/chat/ConnectionLine.svelte`).
//!
//! ```text
//! action   command               parameters   data back
//! read     connection_line_get   (none)       ConnectionLineView, or null when no connection is chosen
//! ```
//!
//! The chat asks in the role *consultation*: the connection given to that
//! role, else the active one (`ai/api/roles.rs`); the line names the same.

pub mod commands;
pub mod view;
