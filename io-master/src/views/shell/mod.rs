//! The view Shell: the frame of the window around every page of a
//! logged-in evaluator (page: `ui/src/views/shell/`, and `ui/src/App.svelte`,
//! which chooses between Start and the frame).
//!
//! ```text
//! element          folder        command              data back
//! the program      program/      program_get          ProgramView: installed, the role, who is logged in,
//!                                                     the pages, the warnings, the version, the folder
//! the user menu    user_menu/    user_menu_log_out    (none)
//! ```
//!
//! The flow of data. The page reads the state of the program at the start
//! and after every change of it (the setup, the login, the logout), and
//! shows the setup, the login or the frame from it:
//!
//! ```text
//! App ──get()──► program_get ──► ProgramView
//!  ▲
//!  └── "done" ◄── setup form, login form (start/), user menu (log out)
//! ```
//!
//! Which page of the frame is open is presentation, kept in the page.

pub mod program;
pub mod user_menu;
