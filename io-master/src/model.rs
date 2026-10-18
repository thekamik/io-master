//! The data of the program as plain Rust types, and the rules of
//! their validity (step 40 of docs/todo.md).
//!
//! **No I/O here:** no database, no network, no file. Every other
//! folder may use `model/`; `model/` uses none of them, so the types
//! can be read and audited on their own.
//!
//! ```text
//! ai.rs              a connection to an AI service, the tasks and roles of
//!                    the AI, a conversation
//! analysis.rs        a part of a standard sent to the AI, and what it proposed
//! api_key.rs         an API key, wiped from memory, never shown
//! chats.rs           the chats with the AI
//! installation.rs    the installation: identifier, role; user names
//! standards.rs       the documents of standards
//! text.rs            the rules every text field follows
//! time.rs            the time stamps
//! ```

pub mod ai;
pub mod api_key;
pub mod chats;
pub mod installation;
pub mod standards;
pub mod text;
pub mod time;
