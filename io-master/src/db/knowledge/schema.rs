//! The tables of the knowledge bases of the AI (`model/knowledge.rs`, step
//! 57.2 of docs/todo.md), in the database of an evaluator. **Protection:
//! encrypted by SQLCipher** with the key of the database of the evaluator
//! (`db/encryption/`): the passages hold the text of the sources, which may
//! be licensed standards.
//!
//! ```text
//! knowledge_bases      a base: its name (unique) and what it is for
//! knowledge_sources    a source of a base: its kind, its name, the number of its passages
//! knowledge_passages   the passages of the sources and their index of words (FTS5, BM25)
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
/// `knowledge_passages` is a virtual table of FTS5: it keeps the text of a
/// passage and its index, and beside them, not indexed, its source, page
/// and label; it has no foreign key, so `delete.rs` removes the passages of
/// a source itself. The tokenizer `unicode61` cuts the words at every sign
/// that is no letter or digit and folds the case and the diacritics;
/// `porter` before it reduces an English word to its stem, so that "keys"
/// finds "key" and "generated" finds "generation".
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS knowledge_bases (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE,
        description TEXT NOT NULL,
        created     INTEGER NOT NULL
    );

    -- kind: model/knowledge.rs (SourceKind).
    CREATE TABLE IF NOT EXISTS knowledge_sources (
        id       INTEGER PRIMARY KEY,
        base_id  INTEGER NOT NULL REFERENCES knowledge_bases(id),
        kind     TEXT NOT NULL,
        name     TEXT NOT NULL,
        passages INTEGER NOT NULL,
        added    INTEGER NOT NULL
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_passages USING fts5(
        text,
        source_id UNINDEXED,
        page UNINDEXED,
        label UNINDEXED,
        tokenize = 'porter unicode61 remove_diacritics 2'
    );
";
