//! The tables of the evaluation support (`model/evaluation_support.rs`),
//! in the database of an evaluator. **Protection: encrypted by
//! SQLCipher** with the key of the database of the evaluator
//! (`db/encryption/`). Linked to the documents of OSCAL only by the UUID
//! of a document and the id of an object in it; nothing here is part of
//! an OSCAL document.
//!
//! ```text
//! support_quotes      the passages of the documents of standards an object comes from
//! support_proposals   proposals of the AI waiting for the evaluator
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
pub const SCHEMA: &str = "
    -- found: whether the text was found on its page (business/standards/quote.rs).
    CREATE TABLE IF NOT EXISTS support_quotes (
        id                  INTEGER PRIMARY KEY,
        document_uuid       TEXT NOT NULL,
        object_id           TEXT NOT NULL,
        source_document_id  INTEGER NOT NULL REFERENCES standard_documents (id),
        page                INTEGER NOT NULL,
        text                TEXT NOT NULL,
        found               INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS support_quotes_of ON support_quotes (document_uuid, object_id);

    -- kind: group or control; content: the OSCAL fragment proposed, as
    -- JSON; quotes: its passages, as JSON (source_document_id, page,
    -- text, found); note: the explanation of the AI, never a quote.
    CREATE TABLE IF NOT EXISTS support_proposals (
        id              INTEGER PRIMARY KEY,
        document_uuid   TEXT NOT NULL,
        kind            TEXT NOT NULL,
        parent_id       TEXT,
        content         TEXT NOT NULL,
        quotes          TEXT NOT NULL,
        note            TEXT NOT NULL,
        model           TEXT NOT NULL,
        created         INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER))
    );

    CREATE INDEX IF NOT EXISTS support_proposals_of ON support_proposals (document_uuid);
";
