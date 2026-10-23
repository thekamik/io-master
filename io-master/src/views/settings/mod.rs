//! The view Settings: the password of the evaluator, the connections to
//! AI services, and what the installation is (page: `ui/src/views/settings/`).
//!
//! ```text
//! element               folder               commands
//! the password          password_form/       password_form_change
//! the connections       connection_list/     connection_list_get
//! a connection          connection_form/     connection_form_new, connection_form_get, connection_form_save,
//!                                            connection_form_delete, connection_form_activate,
//!                                            connection_form_field_help
//! its API key           api_key/             api_key_get, api_key_store, api_key_delete
//! its check             connection_check/    connection_check_run
//! ```
//!
//! The tab Program shows the state of the program the frame has already
//! read (`shell/program/`); it has no command of its own.
//!
//! The flow of data on the tab AI. Which connection is chosen is
//! presentation, kept by the page of the view. When the form changed the
//! connections (saved, deleted, made the active one), it tells the view,
//! and the list and the key read their data again:
//!
//! ```text
//! list ──choose(id)──► the view ──► form ──get(id)──► connection_form_get ──► ConnectionView
//!                                ├─► key  ──get(id)──► api_key_get ──► ApiKeyView (whether one is stored, never the key)
//!                                └─► check (emptied)
//! form ──save / delete / activate──► db/ai/connection (through ai/api/connections)
//!   └── "changed" ──► the view ──► list ──get()──► connection_list_get; key ──get(id)──► api_key_get
//! check ──run(id)──► connection_check_run ──► ai/api/checks: one short message, the journal ──► CheckView
//! ```
//!
//! **A key is never sent back to the page**: the page learns only whether
//! one is stored. Adding users is not here: one Linux account holds one
//! installation with one user (docs/architecture.md, section 3).

pub mod api_key;
pub mod connection_check;
pub mod connection_form;
pub mod connection_list;
pub mod password_form;
