//! The element "check of a connection" of the view Settings, tab AI
//! (page: `ui/src/views/settings/ConnectionCheck.svelte`).
//!
//! ```text
//! action   command                parameters   data back
//! check    connection_check_run   id           CheckView: what went out, what came back, what the paths found
//! ```
//!
//! Sends one short message over the stored connection (`ai/api/checks.rs`),
//! writes the check into the journal of the AI, and reports it with the
//! key masked everywhere.

pub mod commands;
pub mod view;
