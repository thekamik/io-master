//! `objects/`: every model with the functions that work on it, a folder per
//! object, its files named by the object and their role (user, 2026-11-01):
//! <object>_model.rs (the structure), <object>_store.rs (create, read,
//! update), <object>_rules.rs (the rules of the data), <object>_ai.rs (the
//! work of the AI); a big role becomes a folder of the same name.

// AiError, the error of every work of the AI.
pub mod ai_error;
// DomainError, the error of the rules of the data.
pub mod error;
// The one rule of a text the user gives: trimmed, within its limit.
pub mod text;
// The tables of each database, given to infrastructure/storage/ when a
// database is opened.
pub mod ai;
pub mod evaluation;
pub mod schema;
pub mod session;
pub mod user;
