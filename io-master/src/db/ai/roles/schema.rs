//! The table of the roles of the AI, in the database of an evaluator:
//! which connection a role uses. The roles themselves are fixed by the
//! program (`ai/api/roles.rs`). **Protection: encrypted by SQLCipher**
//! with the key of the database of the evaluator (`db/encryption/`).
//!
//! ```text
//! ai_roles   role -> connection; no row, or no connection: the role
//!            uses the active connection
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). A connection that is deleted leaves
/// its roles without one (`ON DELETE SET NULL`): they use the active
/// connection again.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS ai_roles (
        role            TEXT PRIMARY KEY,
        connection_id   INTEGER REFERENCES ai_connections (id) ON DELETE SET NULL
    );
";
