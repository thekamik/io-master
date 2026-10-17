//! The journal of an evaluation: every step, only ever added to
//! (`schema.rs`, table `evaluation_journal` and its two triggers).

pub mod create;
pub mod read;
pub mod rows;
