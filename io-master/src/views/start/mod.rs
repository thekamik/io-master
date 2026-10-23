//! The view Start: what the window shows before anybody is logged in
//! (page: `ui/src/views/start/`).
//!
//! ```text
//! element      folder          command               shown
//! the setup    setup_form/     setup_form_install    at the first start
//! the login    login_form/     login_form_log_in     when nobody is logged in
//! ```
//!
//! The flow of data. Neither element gives data back: after the setup or
//! the login the page asks for the state of the program again
//! (`shell/program/`), which then shows the frame of the window:
//!
//! ```text
//! setup form ──install(name, password, role)──► setup_form_install ──► installation/: made, logged in
//! login form ──log_in(name, password)─────────► login_form_log_in  ──► installation/: logged in
//!      └── "done" ──► the page ──► program_get ──► ProgramView: who is logged in, the pages
//! ```
//!
//! There is no one-time setup code, unlike in the web interface: the
//! code protected a network port against other programs and accounts of
//! the computer, and the window has no port. Only the person in front of
//! the window can make the setup.

pub mod login_form;
pub mod setup_form;
