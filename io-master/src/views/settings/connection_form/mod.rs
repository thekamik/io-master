//! The element "connection form" of the view Settings, tab AI: one
//! connection to an AI service, with the presets of well-known services
//! and the help of a chatbot for its fields
//! (page: `ui/src/views/settings/ConnectionForm.svelte`).
//!
//! ```text
//! action      command                      parameters               data back
//! new         connection_form_new          (none)                   the template and the presets
//! read        connection_form_get          id                       ConnectionView
//! save        connection_form_save         id or null, connection   SavedView: the id, the sentence for the page
//! delete      connection_form_delete       id                       (none); its API key with it
//! activate    connection_form_activate     id                       (none); the chat uses it from now on
//! AI help     connection_form_field_help   field, connection        the instruction for a chatbot
//! ```
//!
//! The program knows no provider: every connection says where the service
//! is, how a request looks and where in the answer the text is
//! (`model/ai.rs`, checked by the core). Saving needs no answer of the
//! service (a local model may be switched off while it is set up). A
//! preset only fills the fields in the page, as if they were typed. "AI
//! help" gives an instruction for a chatbot of the evaluator's choice; it
//! holds no value of a header and no key (`ai/presets/field_help.rs`).

pub mod commands;
pub mod view;
