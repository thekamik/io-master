//! The desktop window: the commands, the state, the serializer.

pub mod account_actions;
pub mod ai_config_actions;
pub mod blocking;
pub mod chat_actions;
pub mod commands;
pub mod error;
pub mod project_actions;
pub mod scan_actions;
pub mod scheme_actions;
pub mod serializer;
pub mod standard_actions;
pub mod state;
#[cfg(test)]
pub mod test_support;
pub mod user_data;
