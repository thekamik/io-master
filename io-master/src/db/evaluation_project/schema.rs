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
//! evaluation_tasks      its tasks, each a control of one of its catalogs, in order
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

    -- One task per control in a project.
    CREATE TABLE IF NOT EXISTS evaluation_tasks (
        uuid            TEXT PRIMARY KEY,
        project_uuid    TEXT NOT NULL,
        catalog_uuid    TEXT NOT NULL,
        control_id      TEXT NOT NULL,
        position        INTEGER NOT NULL,
        UNIQUE (project_uuid, catalog_uuid, control_id)
    );
";
