//! The tables of the conversions of the documents of standards into
//! catalogs of OSCAL (`model/conversion.rs`, step 50.2 of docs/todo.md), in
//! the database of an evaluator. **Protection: encrypted by SQLCipher** with
//! the key of the database of the evaluator (`db/encryption/`). Linked to
//! the catalog only by its UUID; nothing here is part of a document of OSCAL.
//!
//! ```text
//! conversions         one row per catalog in conversion: its stage, the pages
//!                     of its table of contents
//! conversion_groups   the pages and the state of every group whose controls
//!                     the AI reads
//! ```
//!
//! A catalog without a row in `conversions` was imported, not converted,
//! and counts as finished.

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`).
pub const SCHEMA: &str = "
    -- stage: model/conversion.rs, Stage::name. The pages of the table of
    -- contents are NULL until they are known.
    CREATE TABLE IF NOT EXISTS conversions (
        catalog_uuid        TEXT PRIMARY KEY,
        stage               TEXT NOT NULL,
        contents_document   INTEGER,
        contents_first      INTEGER,
        contents_last       INTEGER
    );

    -- state: model/conversion.rs, GroupState::name; position: the order of
    -- the document. The pages are NULL until they are known.
    CREATE TABLE IF NOT EXISTS conversion_groups (
        catalog_uuid        TEXT NOT NULL,
        group_id            TEXT NOT NULL,
        position            INTEGER NOT NULL,
        document_id         INTEGER,
        first_page          INTEGER,
        last_page           INTEGER,
        state               TEXT NOT NULL,
        PRIMARY KEY (catalog_uuid, group_id)
    );
";
