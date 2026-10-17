//! The tables of the test projects, in the database of an evaluator
//! (steps 16 to 27 of docs/todo.md). **Protection: encrypted by
//! SQLCipher** with the key of the database of the evaluator
//! (`encryption/`).
//!
//! ```text
//! projects            name, version, description, author, ready
//! project_documents   the documents of the standard a project uses
//! schemes             the schemes (chapters) of a project
//! tiles, tile_sources the tiles of the schemes and their quotes
//! links               the links of a project
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`).
pub const SCHEMA: &str = "
    -- A test project: the map of a standard (model/test_projects.rs).
    CREATE TABLE IF NOT EXISTS projects (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE,
        version     TEXT NOT NULL,
        description TEXT NOT NULL,
        author      TEXT NOT NULL,
        ready       INTEGER NOT NULL
    );

    -- The documents a project uses, in the order the user gave them.
    CREATE TABLE IF NOT EXISTS project_documents (
        project_id  INTEGER NOT NULL REFERENCES projects (id),
        document_id INTEGER NOT NULL REFERENCES standard_documents (id),
        position    INTEGER NOT NULL,
        PRIMARY KEY (project_id, document_id)
    );

    -- A scheme: one chapter of the standard. state: proposed or
    -- accepted; document_id, first_page, last_page: the pages of the
    -- chapter, all NULL when not known; position: the place in the
    -- list of the project.
    CREATE TABLE IF NOT EXISTS schemes (
        id          INTEGER PRIMARY KEY,
        project_id  INTEGER NOT NULL REFERENCES projects (id),
        number      TEXT NOT NULL,
        name        TEXT NOT NULL,
        state       TEXT NOT NULL,
        document_id INTEGER REFERENCES standard_documents (id),
        first_page  INTEGER,
        last_page   INTEGER,
        position    INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS schemes_of_project ON schemes (project_id);

    -- kind: input, requirement or output; state: proposed or accepted;
    -- external_*: an input described in a document not loaded;
    -- sections: the sections of the report of an output, by name,
    -- separated by commas (model/test_projects.rs).
    CREATE TABLE IF NOT EXISTS tiles (
        id                  INTEGER PRIMARY KEY,
        scheme_id           INTEGER NOT NULL REFERENCES schemes (id),
        kind                TEXT NOT NULL,
        identifier          TEXT NOT NULL,
        title               TEXT NOT NULL,
        description         TEXT NOT NULL,
        place               TEXT NOT NULL,
        state               TEXT NOT NULL,
        external_document   TEXT,
        external_place      TEXT,
        sections            TEXT NOT NULL,
        instructions        TEXT NOT NULL,
        x                   REAL NOT NULL,
        y                   REAL NOT NULL
    );

    CREATE INDEX IF NOT EXISTS tiles_of_scheme ON tiles (scheme_id);

    -- found: whether the quote was found on its page (standards/quote.rs).
    CREATE TABLE IF NOT EXISTS tile_sources (
        id          INTEGER PRIMARY KEY,
        tile_id     INTEGER NOT NULL REFERENCES tiles (id),
        document_id INTEGER NOT NULL REFERENCES standard_documents (id),
        page        INTEGER NOT NULL,
        quote       TEXT NOT NULL,
        found       INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS sources_of_tile ON tile_sources (tile_id);

    -- A link belongs to the project: its two tiles may be in two
    -- schemes of it.
    CREATE TABLE IF NOT EXISTS links (
        id          INTEGER PRIMARY KEY,
        project_id  INTEGER NOT NULL REFERENCES projects (id),
        from_tile   INTEGER NOT NULL REFERENCES tiles (id),
        to_tile     INTEGER NOT NULL REFERENCES tiles (id),
        UNIQUE (from_tile, to_tile)
    );
";
