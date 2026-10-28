//! The table of the documents of OSCAL, in the database of an
//! evaluator. **Protection: encrypted by SQLCipher** with the key of the
//! database of the evaluator (`db/encryption/`); a document is
//! overwritten in the file when it is deleted (`delete.rs`).
//!
//! **A document is kept as its JSON, as OSCAL defines it** (decided by
//! the user, 2026-10-18), checked against the schema of NIST at every
//! write (`create.rs`, `update.rs`); the other columns repeat what
//! identifies it, for the lists. What the program adds to a document
//! (the evaluation support) is in tables of its own, linked by UUIDs.
//!
//! ```text
//! oscal_documents   one row per document: its UUID, model, title, versions, JSON
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
pub const SCHEMA: &str = "
    -- model: catalog, profile, mapping-collection, assessment-plan,
    -- assessment-results (model/oscal.rs); json: the document, checked
    -- against the schema of NIST when it was written.
    CREATE TABLE IF NOT EXISTS oscal_documents (
        uuid            TEXT PRIMARY KEY,
        model           TEXT NOT NULL,
        title           TEXT NOT NULL,
        version         TEXT NOT NULL,
        oscal_version   TEXT NOT NULL,
        last_modified   TEXT NOT NULL,
        json            TEXT NOT NULL,
        stored          INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER))
    );
";
