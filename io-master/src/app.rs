//! What the whole window shares (Tauri 2): the state the commands of the
//! views reach, and their errors. The commands themselves are in the
//! folders of the elements of the views (`views/`).
//!
//! ```text
//! state.rs        the installation, the session of the evaluator, the stop signals
//! error.rs        every error of a command, in kinds the page tells apart
//! serializer/     how an error reaches the page (serde)
//! ```

pub mod error;
pub mod serializer;
pub mod state;
#[cfg(test)]
pub mod test_support;
