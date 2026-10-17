//! The tables of the database of an evaluator that belong to no other
//! area: settings and secrets. **Protection: encrypted by SQLCipher**
//! with the key of the database of the evaluator (`encryption/`).
//!
//! ```text
//! settings   name -> value: the settings of the evaluator, for
//!            example of the scanner (devices/scan_settings.rs)
//! secrets    name -> value: secrets of the evaluator, such as the
//!            API keys of the AI connections (db/ai/connection/);
//!            kept apart from the settings so that they are never
//!            listed or shown with them
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`database.rs`).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS settings (
        name    TEXT PRIMARY KEY,
        value   TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS secrets (
        name    TEXT PRIMARY KEY,
        value   TEXT NOT NULL
    );
";
