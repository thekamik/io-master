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
//! knowledge_agents     an agent: its name (unique), instructions, glossary, limits (step 57.3)
//! knowledge_agent_bases the bases an agent draws on
//! knowledge_trials     the last question of the proving ground of an agent, and its answer
//! knowledge_trial_passages  the passages it was given, copied (a source may go later)
//! knowledge_trial_quotes    the quotes of its answer, and whether they were found
//! knowledge_attachments     the agent of a master project, a catalog or a chat (step 57.6)
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

    CREATE TABLE IF NOT EXISTS knowledge_agents (
        id           INTEGER PRIMARY KEY,
        name         TEXT NOT NULL UNIQUE,
        instructions TEXT NOT NULL,
        glossary     TEXT NOT NULL,
        passages     INTEGER NOT NULL,
        characters   INTEGER NOT NULL,
        created      INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS knowledge_agent_bases (
        agent_id INTEGER NOT NULL REFERENCES knowledge_agents(id),
        base_id  INTEGER NOT NULL REFERENCES knowledge_bases(id),
        PRIMARY KEY (agent_id, base_id)
    );

    CREATE TABLE IF NOT EXISTS knowledge_trials (
        agent_id INTEGER PRIMARY KEY REFERENCES knowledge_agents(id),
        question TEXT NOT NULL,
        answer   TEXT NOT NULL,
        asked    INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS knowledge_trial_passages (
        agent_id   INTEGER NOT NULL REFERENCES knowledge_agents(id),
        position   INTEGER NOT NULL,
        passage_id INTEGER NOT NULL,
        source     TEXT NOT NULL,
        page       INTEGER,
        label      TEXT NOT NULL,
        text       TEXT NOT NULL,
        score      REAL NOT NULL
    );

    -- place: model/knowledge.rs (AgentPlace); owner: the UUID or the id of
    -- the object of that place.
    CREATE TABLE IF NOT EXISTS knowledge_attachments (
        place    TEXT NOT NULL,
        owner    TEXT NOT NULL,
        agent_id INTEGER NOT NULL REFERENCES knowledge_agents(id),
        PRIMARY KEY (place, owner)
    );

    CREATE TABLE IF NOT EXISTS knowledge_trial_quotes (
        agent_id INTEGER NOT NULL REFERENCES knowledge_agents(id),
        position INTEGER NOT NULL,
        passage  INTEGER NOT NULL,
        quote    TEXT NOT NULL,
        found    INTEGER NOT NULL
    );
";
