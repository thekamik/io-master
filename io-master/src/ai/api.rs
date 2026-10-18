//! The only way to the AI, in the form of REST: resources and operations.

pub mod checks;
pub mod completions;
pub mod connection;
pub mod connections;
// The communication with the provider: only `ai/api/` reaches
// it; the rest of the program asks through the files above.
pub(in crate::ai::api) mod provider;
pub mod roles;
pub mod tasks;
#[cfg(test)]
pub mod test_support;
