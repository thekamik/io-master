//! The table of the AI connections, in the database of an evaluator.
//! **Protection: encrypted by SQLCipher** with the key of the database
//! of the evaluator (`db/encryption/`). The API keys are not here:
//! they are secrets (`db/user/`, table `secrets`, as
//! `ai.connection.<id>.api_key`); the active connection is the setting
//! `ai.active_connection`.
//!
//! ```text
//! ai_connections   the connections to AI services, as the evaluator
//!                  set them up on the tab AI of Settings
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`).
pub const SCHEMA: &str = "
    -- The fields of ai/connection.rs, as they were entered; the API
    -- key is a secret ('ai.connection.<id>.api_key'), key_origin the
    -- address it is bound to.
    CREATE TABLE IF NOT EXISTS ai_connections (
        id                  INTEGER PRIMARY KEY,
        name                TEXT NOT NULL UNIQUE,
        url                 TEXT NOT NULL,
        model               TEXT NOT NULL,
        ca_certificate      TEXT NOT NULL,
        headers             TEXT NOT NULL,
        body                TEXT NOT NULL,
        user_role           TEXT NOT NULL,
        model_role          TEXT NOT NULL,
        system_role         TEXT NOT NULL,
        system_prompt       TEXT NOT NULL,
        answer              TEXT NOT NULL,
        text_path           TEXT NOT NULL,
        stop_path           TEXT NOT NULL,
        cut_reasons         TEXT NOT NULL,
        refused_reasons     TEXT NOT NULL,
        error_path          TEXT NOT NULL,
        model_path          TEXT NOT NULL,
        input_tokens_path   TEXT NOT NULL,
        output_tokens_path  TEXT NOT NULL,
        done_marker         TEXT NOT NULL,
        key_origin          TEXT,
        message_template    TEXT NOT NULL DEFAULT '{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}'
    );
";
