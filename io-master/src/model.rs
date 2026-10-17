//! The data of the program as plain Rust types, and the rules of
//! their validity (step 40 of docs/todo.md).
//!
//! **No I/O here:** no database, no network, no file. Every other
//! folder may use `model/`; `model/` uses none of them, so the types
//! can be read and audited on their own.
//!
//! ```text
//! standards.rs       the documents of standards
//! test_projects.rs   the test projects: schemes, tiles, links
//!   model/test_projects/rules.rs   the rules of their fields and links
//! evaluations.rs     the evaluation projects: results, proposals, journal
//! text.rs            the rules every text field follows
//! time.rs            the time stamps
//! ```

pub mod evaluations;
pub mod standards;
pub mod test_projects;
pub mod text;
pub mod time;
