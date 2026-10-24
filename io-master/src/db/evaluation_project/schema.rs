//! The tables of the master projects of the evaluations
//! (`model/evaluation_project.rs`, step 51 of docs/todo.md), in the database
//! of an evaluator. **Protection: encrypted by SQLCipher** with the key of the
//! database of the evaluator (`db/encryption/`). Linked to the catalogs of
//! OSCAL only by their UUIDs and the ids of their controls; nothing here is
//! part of a document of OSCAL.
//!
//! ```text
//! evaluation_projects   a master project: its title and description
//! project_catalogs      the catalogs a project uses, in the order chosen
//! standard_outputs      the template of the outputs of its tasks (step 51.3), in order
//! evaluation_tasks      its tasks, each a control of one of its catalogs, in order
//! task_blocks           the blocks of the scheme of a task (step 51.2): its
//!                       requirement, its inputs, its outputs, placed in the drawing
//! task_connections      the connections of the blocks of a task
//! ```

/// The tables, created with `IF NOT EXISTS` whenever the database is
/// opened (`db/user/database.rs`). Times are seconds since 1970 (UTC).
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS evaluation_projects (
        uuid            TEXT PRIMARY KEY,
        title           TEXT NOT NULL,
        description     TEXT NOT NULL,
        modified        INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS project_catalogs (
        project_uuid    TEXT NOT NULL,
        catalog_uuid    TEXT NOT NULL,
        position        INTEGER NOT NULL,
        PRIMARY KEY (project_uuid, catalog_uuid)
    );

    -- The template of the outputs of a project: format and condition as in
    -- task_blocks; a new task gets a copy of them as its outputs.
    CREATE TABLE IF NOT EXISTS standard_outputs (
        uuid            TEXT PRIMARY KEY,
        project_uuid    TEXT NOT NULL,
        position        INTEGER NOT NULL,
        name            TEXT NOT NULL,
        instructions    TEXT NOT NULL,
        target          TEXT NOT NULL,
        format          TEXT NOT NULL,
        condition       TEXT NOT NULL
    );

    -- One task per control in a project.
    CREATE TABLE IF NOT EXISTS evaluation_tasks (
        uuid            TEXT PRIMARY KEY,
        project_uuid    TEXT NOT NULL,
        catalog_uuid    TEXT NOT NULL,
        control_id      TEXT NOT NULL,
        position        INTEGER NOT NULL,
        UNIQUE (project_uuid, catalog_uuid, control_id)
    );

    -- kind, format, condition: model/task_scheme.rs (BlockKind, OutputFormat,
    -- OutputCondition). The fields a kind does not use are empty.
    CREATE TABLE IF NOT EXISTS task_blocks (
        uuid            TEXT PRIMARY KEY,
        task_uuid       TEXT NOT NULL,
        kind            TEXT NOT NULL,
        x               INTEGER NOT NULL,
        y               INTEGER NOT NULL,
        name            TEXT NOT NULL,
        evidence        TEXT NOT NULL,
        instructions    TEXT NOT NULL,
        target          TEXT NOT NULL,
        format          TEXT NOT NULL,
        condition       TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS task_connections (
        task_uuid       TEXT NOT NULL,
        from_block      TEXT NOT NULL,
        to_block        TEXT NOT NULL,
        PRIMARY KEY (from_block, to_block)
    );
";
