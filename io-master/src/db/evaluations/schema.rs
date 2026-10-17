//! The tables of the evaluation projects, in the database of an
//! evaluator (step 29 of docs/todo.md). **Protection: encrypted by
//! SQLCipher** with the key of the database of the evaluator
//! (`encryption/`); an evaluation is overwritten in the file when it
//! is deleted (`delete.rs`).
//!
//! The schemes, tiles, sources and links are a frozen copy of the
//! accepted part of a test project, in tables of their own: a test
//! project may change or be deleted later, the evaluation may not.
//!
//! ```text
//! evaluations              the evaluation: product, origin, ...
//! evaluation_documents     the documents of the standard it uses
//! evaluation_schemes, evaluation_tiles, evaluation_tile_sources,
//! evaluation_links         the frozen copy
//! evaluation_results       the accepted text of a field of a requirement
//! evaluation_proposals     texts of the AI, waiting for the evaluator
//! evaluation_journal       every step; append-only (two triggers)
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
pub const SCHEMA: &str = "
    -- source_*: the test project it was made from, as text.
    CREATE TABLE IF NOT EXISTS evaluations (
        id                  INTEGER PRIMARY KEY,
        name                TEXT NOT NULL UNIQUE,
        product             TEXT NOT NULL,
        product_version     TEXT NOT NULL,
        manufacturer        TEXT NOT NULL,
        evaluator           TEXT NOT NULL,
        source_name         TEXT NOT NULL,
        source_version      TEXT NOT NULL,
        source_author       TEXT NOT NULL,
        source_description  TEXT NOT NULL,
        imported            INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER))
    );

    CREATE TABLE IF NOT EXISTS evaluation_documents (
        evaluation_id   INTEGER NOT NULL REFERENCES evaluations (id),
        document_id     INTEGER NOT NULL REFERENCES standard_documents (id),
        position        INTEGER NOT NULL,
        PRIMARY KEY (evaluation_id, document_id)
    );

    -- As the table schemes, without a state: only accepted schemes
    -- are copied.
    CREATE TABLE IF NOT EXISTS evaluation_schemes (
        id              INTEGER PRIMARY KEY,
        evaluation_id   INTEGER NOT NULL REFERENCES evaluations (id),
        number          TEXT NOT NULL,
        name            TEXT NOT NULL,
        document_id     INTEGER REFERENCES standard_documents (id),
        first_page      INTEGER,
        last_page       INTEGER,
        position        INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS evaluation_schemes_of ON evaluation_schemes (evaluation_id);

    -- As the table tiles, without a state: only accepted tiles are copied.
    CREATE TABLE IF NOT EXISTS evaluation_tiles (
        id                  INTEGER PRIMARY KEY,
        evaluation_id       INTEGER NOT NULL REFERENCES evaluations (id),
        scheme_id           INTEGER NOT NULL REFERENCES evaluation_schemes (id),
        kind                TEXT NOT NULL,
        identifier          TEXT NOT NULL,
        title               TEXT NOT NULL,
        description         TEXT NOT NULL,
        place               TEXT NOT NULL,
        external_document   TEXT,
        external_place      TEXT,
        sections            TEXT NOT NULL,
        instructions        TEXT NOT NULL,
        x                   REAL NOT NULL,
        y                   REAL NOT NULL
    );

    CREATE INDEX IF NOT EXISTS evaluation_tiles_of ON evaluation_tiles (evaluation_id);

    CREATE TABLE IF NOT EXISTS evaluation_tile_sources (
        id          INTEGER PRIMARY KEY,
        tile_id     INTEGER NOT NULL REFERENCES evaluation_tiles (id),
        document_id INTEGER NOT NULL REFERENCES standard_documents (id),
        page        INTEGER NOT NULL,
        quote       TEXT NOT NULL,
        found       INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS evaluation_sources_of ON evaluation_tile_sources (tile_id);

    CREATE TABLE IF NOT EXISTS evaluation_links (
        id              INTEGER PRIMARY KEY,
        evaluation_id   INTEGER NOT NULL REFERENCES evaluations (id),
        from_tile       INTEGER NOT NULL REFERENCES evaluation_tiles (id),
        to_tile         INTEGER NOT NULL REFERENCES evaluation_tiles (id),
        UNIQUE (from_tile, to_tile)
    );

    -- The accepted text of one field of a requirement. field:
    -- findings, analysis, assessment, vulnerabilities; origin:
    -- evaluator, ai, ai_corrected (evaluation/model.rs).
    CREATE TABLE IF NOT EXISTS evaluation_results (
        tile_id     INTEGER NOT NULL REFERENCES evaluation_tiles (id),
        field       TEXT NOT NULL,
        text        TEXT NOT NULL,
        origin      TEXT NOT NULL,
        PRIMARY KEY (tile_id, field)
    );

    -- A text of the AI, waiting for the evaluator.
    CREATE TABLE IF NOT EXISTS evaluation_proposals (
        id          INTEGER PRIMARY KEY,
        tile_id     INTEGER NOT NULL REFERENCES evaluation_tiles (id),
        field       TEXT NOT NULL,
        text        TEXT NOT NULL,
        model       TEXT NOT NULL,
        created     INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER))
    );

    CREATE INDEX IF NOT EXISTS evaluation_proposals_of ON evaluation_proposals (tile_id);

    -- The journal (P3 of architecture.md). actor: evaluator or model
    -- (with model); action: evaluation/model.rs. evaluation_id
    -- and tile_id carry no REFERENCES on purpose: the journal outlives
    -- the rows it names until the whole evaluation is deleted.
    CREATE TABLE IF NOT EXISTS evaluation_journal (
        id              INTEGER PRIMARY KEY,
        evaluation_id   INTEGER NOT NULL,
        time            INTEGER NOT NULL DEFAULT (CAST(strftime('%s', 'now') AS INTEGER)),
        actor           TEXT NOT NULL,
        model           TEXT,
        action          TEXT NOT NULL,
        tile_id         INTEGER,
        field           TEXT,
        detail          TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS evaluation_journal_of ON evaluation_journal (evaluation_id, id);

    -- The journal is only ever added to: an entry is never changed,
    -- and removed only together with its whole evaluation (after the
    -- row in evaluations is gone). Kept by the database itself, so
    -- that not even a mistake of the program rewrites the history.
    CREATE TRIGGER IF NOT EXISTS evaluation_journal_unchanged BEFORE UPDATE ON evaluation_journal
    BEGIN
        SELECT RAISE(ABORT, 'an entry of the journal of an evaluation is never changed');
    END;

    CREATE TRIGGER IF NOT EXISTS evaluation_journal_kept BEFORE DELETE ON evaluation_journal
    WHEN EXISTS (SELECT 1 FROM evaluations WHERE id = OLD.evaluation_id)
    BEGIN
        SELECT RAISE(ABORT, 'the journal is removed only with its whole evaluation');
    END;
";
