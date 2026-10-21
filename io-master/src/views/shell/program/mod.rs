//! The element "program" of the view Shell: the state of the program the
//! page needs to show itself (page: `ui/src/App.svelte` and
//! `ui/src/views/shell/Shell.svelte`).
//!
//! ```text
//! action   command       parameters   data back
//! read     program_get   (none)       ProgramView
//! ```

pub mod commands;
pub mod view;
