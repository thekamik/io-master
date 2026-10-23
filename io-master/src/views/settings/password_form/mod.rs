//! The element "password form" of the view Settings, tab User
//! (page: `ui/src/views/settings/PasswordForm.svelte`).
//!
//! ```text
//! action   command                parameters                          data back
//! change   password_form_change   current, password, password_repeat  (none)
//! ```
//!
//! Re-encrypts only the 32-byte key of the database with the new password
//! (`installation/accounts.rs`), not the database.

pub mod commands;
