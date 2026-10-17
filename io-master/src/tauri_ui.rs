//! The desktop window (Tauri 2), the thin layer between the business
//! (`business/`) and the page (`ui/`), step 40 of docs/todo.md:
//!
//! ```text
//! commands.rs     every command the page may call: the whole surface
//! commands/       per resource: the session, the form into a type of
//!                 model/, the call of business/, the result into a view
//! serializer/     the views and forms (serde), only here
//! session.rs      the session of the logged-in evaluator, for a command
//! state.rs        the session and the stop signals
//! error.rs        every error of the business as an error of the page
//! blocking.rs     blocking work off the threads of the window
//! ```
//!
//! No rule of the business is here: a rule found in a command is a
//! finding of the review.

pub mod blocking;
pub mod commands;
pub mod error;
pub mod serializer;
pub mod session;
pub mod state;
#[cfg(test)]
pub mod test_support;
