//! The element "list of connections" of the view Settings, tab AI
//! (page: `ui/src/views/settings/ConnectionList.svelte`).
//!
//! ```text
//! action   command               parameters   data back
//! read     connection_list_get   (none)       the connections by name (id, name), and the active one
//! ```
//!
//! Choosing a connection and "New connection" are presentation: the page
//! of the view shows it in the form.

pub mod commands;
pub mod view;
