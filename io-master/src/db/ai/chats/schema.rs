//! The tables of the memory of the chat: the conversations with an AI
//! model and their messages, in the database of an evaluator.
//! **Protection: encrypted by SQLCipher** with the key of the database
//! of the evaluator (`db/encryption/`).
//!
//! ```text
//! chats      the conversations (ai/chat/chats.rs)
//! messages   the messages of the conversations, with their time and
//!            model
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`).
pub const SCHEMA: &str = "
    -- Times are seconds since 1970 (UTC).
    CREATE TABLE IF NOT EXISTS chats (
        id        INTEGER PRIMARY KEY,
        title     TEXT NOT NULL,
        created   INTEGER NOT NULL
    );

    -- speaker: 'evaluator' or 'model'. For a reply of the model:
    -- the model that wrote it, how the reply ended (ai/chat/chats.rs),
    -- an explanation when it failed, and its token counts.
    CREATE TABLE IF NOT EXISTS messages (
        id              INTEGER PRIMARY KEY,
        chat_id         INTEGER NOT NULL REFERENCES chats (id),
        speaker         TEXT NOT NULL,
        text            TEXT NOT NULL,
        created         INTEGER NOT NULL,
        model           TEXT,
        ending          TEXT,
        note            TEXT,
        input_tokens    INTEGER,
        output_tokens   INTEGER
    );

    CREATE INDEX IF NOT EXISTS messages_of_chat ON messages (chat_id, id);
";
