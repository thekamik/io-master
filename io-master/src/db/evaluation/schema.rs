//! The tables of the evaluations of the Evaluation lab
//! (`model/evaluation.rs`, step 52 of docs/todo.md), in the database of an
//! evaluator. **Protection: encrypted by SQLCipher** with the key of the
//! database of the evaluator (`db/encryption/`). Linked to the catalogs of
//! OSCAL only by their UUIDs and the ids of their controls. Every row names
//! its evaluation: a package imported twice into two installations keeps its
//! UUIDs, so the keys hold the evaluation.
//!
//! ```text
//! lab_evaluations          an evaluation: the package it carries out, the product
//! lab_tasks                its tasks (frozen copy of the package), their state
//! lab_blocks               the blocks of their schemes (frozen copy)
//! lab_connections          the connections of the blocks (frozen copy)
//! lab_related              the related elements of the tasks (frozen copy)
//! lab_evidence             the evidence of an input: the bytes as delivered, its text by page
//! lab_messages             the chat of an input
//! lab_fragments            the fragments of the evidence of an input: quotes, notes, accepted or not
//! lab_results              the four results and the verdict of a requirement
//! lab_result_proposals     what the AI proposed for them, not decided yet
//! lab_outputs              the text of an output: accepted, and proposed by the AI
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS lab_evaluations (
        uuid            TEXT PRIMARY KEY,
        package_uuid    TEXT NOT NULL UNIQUE,
        project_title   TEXT NOT NULL,
        scope           TEXT NOT NULL,
        package_name    TEXT NOT NULL,
        evaluator       TEXT NOT NULL,
        note            TEXT NOT NULL,
        product         TEXT NOT NULL,
        product_version TEXT NOT NULL,
        imported        INTEGER NOT NULL
    );

    -- state: model/evaluation.rs (TaskState).
    CREATE TABLE IF NOT EXISTS lab_tasks (
        evaluation_uuid TEXT NOT NULL,
        uuid            TEXT NOT NULL,
        catalog_uuid    TEXT NOT NULL,
        control_id      TEXT NOT NULL,
        label           TEXT NOT NULL,
        title           TEXT NOT NULL,
        position        INTEGER NOT NULL,
        state           TEXT NOT NULL,
        PRIMARY KEY (evaluation_uuid, uuid)
    );

    -- kind, format, condition: model/task_scheme.rs.
    CREATE TABLE IF NOT EXISTS lab_blocks (
        evaluation_uuid TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        uuid            TEXT NOT NULL,
        kind            TEXT NOT NULL,
        x               INTEGER NOT NULL,
        y               INTEGER NOT NULL,
        name            TEXT NOT NULL,
        evidence        TEXT NOT NULL,
        instructions    TEXT NOT NULL,
        target          TEXT NOT NULL,
        format          TEXT NOT NULL,
        condition       TEXT NOT NULL,
        PRIMARY KEY (evaluation_uuid, uuid)
    );

    CREATE TABLE IF NOT EXISTS lab_connections (
        evaluation_uuid TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        from_block      TEXT NOT NULL,
        to_block        TEXT NOT NULL,
        PRIMARY KEY (evaluation_uuid, from_block, to_block)
    );

    CREATE TABLE IF NOT EXISTS lab_related (
        evaluation_uuid TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        catalog_uuid    TEXT NOT NULL,
        control_id      TEXT NOT NULL,
        label           TEXT NOT NULL,
        title           TEXT NOT NULL,
        position        INTEGER NOT NULL
    );

    -- kind: model/evaluation.rs (EvidenceKind); pages: the text of every
    -- page, joined by the form feed (U+000C).
    CREATE TABLE IF NOT EXISTS lab_evidence (
        evaluation_uuid TEXT NOT NULL,
        uuid            TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        block_uuid      TEXT NOT NULL,
        kind            TEXT NOT NULL,
        name            TEXT NOT NULL,
        content         BLOB NOT NULL,
        pages           TEXT NOT NULL,
        added           INTEGER NOT NULL,
        PRIMARY KEY (evaluation_uuid, uuid)
    );

    -- speaker: evaluator or model.
    CREATE TABLE IF NOT EXISTS lab_messages (
        evaluation_uuid TEXT NOT NULL,
        block_uuid      TEXT NOT NULL,
        position        INTEGER NOT NULL,
        speaker         TEXT NOT NULL,
        text            TEXT NOT NULL,
        PRIMARY KEY (evaluation_uuid, block_uuid, position)
    );

    -- page: counted from 1, NULL when not known.
    CREATE TABLE IF NOT EXISTS lab_fragments (
        evaluation_uuid TEXT NOT NULL,
        uuid            TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        block_uuid      TEXT NOT NULL,
        evidence_uuid   TEXT NOT NULL,
        page            INTEGER,
        quote           TEXT NOT NULL,
        note            TEXT NOT NULL,
        found           INTEGER NOT NULL,
        accepted        INTEGER NOT NULL,
        by_ai           INTEGER NOT NULL,
        PRIMARY KEY (evaluation_uuid, uuid)
    );

    -- verdict: model/evaluation.rs (Verdict), NULL when not set.
    CREATE TABLE IF NOT EXISTS lab_results (
        evaluation_uuid TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        findings        TEXT NOT NULL,
        analysis        TEXT NOT NULL,
        assessment      TEXT NOT NULL,
        vulnerabilities TEXT NOT NULL,
        verdict         TEXT,
        PRIMARY KEY (evaluation_uuid, task_uuid)
    );

    CREATE TABLE IF NOT EXISTS lab_result_proposals (
        evaluation_uuid TEXT NOT NULL,
        task_uuid       TEXT NOT NULL,
        findings        TEXT NOT NULL,
        analysis        TEXT NOT NULL,
        assessment      TEXT NOT NULL,
        vulnerabilities TEXT NOT NULL,
        verdict         TEXT,
        reason          TEXT NOT NULL,
        PRIMARY KEY (evaluation_uuid, task_uuid)
    );

    -- proposal: what the AI proposed, NULL when decided.
    CREATE TABLE IF NOT EXISTS lab_outputs (
        evaluation_uuid TEXT NOT NULL,
        block_uuid      TEXT NOT NULL,
        text            TEXT NOT NULL,
        proposal        TEXT,
        PRIMARY KEY (evaluation_uuid, block_uuid)
    );
";
