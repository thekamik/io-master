//! The table of the instructions of the tasks of the AI, as the
//! evaluator changed them, in the database of an evaluator. A task
//! without a row uses its default instructions (`ai/api/tasks.rs`).
//! **Protection: encrypted by SQLCipher** with the key of the database
//! of the evaluator (`db/encryption/`).
//!
//! ```text
//! ai_instructions   task -> the instructions to the model
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_instructions (
        task    TEXT PRIMARY KEY,
        text    TEXT NOT NULL
    );
";
