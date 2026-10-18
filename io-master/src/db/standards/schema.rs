//! The tables of the documents of standards, in the database of an
//! evaluator (steps 16 to 22 of docs/todo.md). **Protection: encrypted
//! by SQLCipher** with the key of the database of the evaluator
//! (`encryption/`); the text of a licensed standard is overwritten in
//! the file when it is deleted (`delete.rs`).
//!
//! ```text
//! standard_documents   a document: the PDF and what the user told about it
//! standard_pages       the text of its pages
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
pub const SCHEMA: &str = "
    -- A document of a standard: the PDF and what the user told about
    -- it; the text of its pages in standard_pages.
    CREATE TABLE IF NOT EXISTS standard_documents (
        id          INTEGER PRIMARY KEY,
        title       TEXT NOT NULL,
        part        TEXT NOT NULL,
        version     TEXT NOT NULL,
        file_name   TEXT NOT NULL,
        sha256      TEXT NOT NULL UNIQUE,
        page_count  INTEGER NOT NULL,
        pdf         BLOB NOT NULL,
        added       INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER))
    );

    CREATE TABLE IF NOT EXISTS standard_pages (
        document_id INTEGER NOT NULL REFERENCES standard_documents (id),
        number      INTEGER NOT NULL,
        text        TEXT NOT NULL,
        PRIMARY KEY (document_id, number)
    );
";
