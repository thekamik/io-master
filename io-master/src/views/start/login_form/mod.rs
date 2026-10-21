//! The element "login form" of the view Start
//! (page: `ui/src/views/start/LoginForm.svelte`).
//!
//! ```text
//! action   command             parameters       data back
//! log in   login_form_log_in   name, password   (none)
//! ```
//!
//! Opens the session of the evaluator (`installation/accounts.rs`,
//! `session.rs`): the database of the evaluator is opened with the key
//! that the password and the secret of the application unlock, and the
//! key is wiped.

pub mod commands;
