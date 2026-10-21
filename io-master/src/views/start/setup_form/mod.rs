//! The element "setup form" of the view Start
//! (page: `ui/src/views/start/SetupForm.svelte`).
//!
//! ```text
//! action    command              parameters                                          data back
//! install   setup_form_install   name, password, password_repeat, role, role_confirmed   (none)
//! ```
//!
//! Makes the installation with its role and its one user
//! (`installation/lifecycle.rs`), and logs that user in.

pub mod commands;
