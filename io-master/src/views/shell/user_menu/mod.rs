//! The element "user menu" of the view Shell: the logged-in evaluator at
//! the foot of the sidebar, and the logout
//! (page: `ui/src/views/shell/UserMenu.svelte`).
//!
//! ```text
//! action    command             parameters   data back
//! log out   user_menu_log_out   (none)       (none)
//! ```
//!
//! The logout ends the session (`app/state.rs`): a reply being
//! written and a long work are stopped, and the database closes and the
//! key is gone when the last work lets the session go.

pub mod commands;
