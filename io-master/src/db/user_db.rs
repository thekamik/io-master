//! The database of one evaluator: `users/<id>.db` in the data
//! directory.
//!
//! Everything an evaluator works with is kept here: the API key,
//! the chats, the settings, later the results. Its key is random
//! and stored only encrypted with the password of the evaluator
//! and the secret of the device (`keys.rs`), so nobody else, not
//! even another evaluator of the same workstation, can open it.
//!
//! ```text
//! settings   name -> value: the settings of the evaluator, for
//!            example of the scanner (devices/scan_settings.rs)
//! secrets    name -> value: secrets of the evaluator, such as the
//!            API keys of the AI connections (ai/connections.rs);
//!            kept apart from the settings so that they are never
//!            listed or shown with them
//! standard_documents, standard_pages   the documents of standards
//!            and the text of their pages (evaluation/documents.rs)
//! projects, project_documents, schemes, tiles, tile_sources, links
//!            the test projects and their schemes (evaluation/
//!            projects.rs, evaluation/schemes.rs)
//! ai_connections   the connections to AI services, as the evaluator
//!            set them up on the tab AI of Settings (ai/connections.rs)
//! chats      the conversations with an AI model (ai/chats.rs)
//! messages   the messages of the conversations, with their time
//!            and model
//! ```
//!
//! Each step of `docs/todo.md` that keeps data of an evaluator adds
//! its own tables. Tables are created with `IF NOT EXISTS` when the
//! database is opened, so a database made by an earlier version
//! gets the tables it lacks.

use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use zeroize::Zeroizing;

use super::database_key::DatabaseKey;
use super::db::{self, DbError};

/// Name of the folder of the databases of the evaluators.
pub const DIRECTORY: &str = "users";

/// The tables of the database.
const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS settings (
        name    TEXT PRIMARY KEY,
        value   TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS secrets (
        name    TEXT PRIMARY KEY,
        value   TEXT NOT NULL
    );

    -- Times are seconds since 1970 (UTC).
    CREATE TABLE IF NOT EXISTS chats (
        id        INTEGER PRIMARY KEY,
        title     TEXT NOT NULL,
        created   INTEGER NOT NULL
    );

    -- speaker: 'evaluator' or 'model'. For a reply of the model:
    -- the model that wrote it, how the reply ended (ai/chats.rs),
    -- an explanation when it failed, and its token counts.
    CREATE TABLE IF NOT EXISTS messages (
        id              INTEGER PRIMARY KEY,
        chat_id         INTEGER NOT NULL REFERENCES chats (id),
        speaker         TEXT NOT NULL,
        text            TEXT NOT NULL,
        created         INTEGER NOT NULL,
        model           TEXT,
        ending          TEXT,
        note            TEXT,
        input_tokens    INTEGER,
        output_tokens   INTEGER
    );

    CREATE INDEX IF NOT EXISTS messages_of_chat ON messages (chat_id, id);

    -- The fields of ai/connection.rs, as they were entered; the API
    -- key is a secret ('ai.connection.<id>.api_key'), key_origin the
    -- address it is bound to.
    CREATE TABLE IF NOT EXISTS ai_connections (
        id                  INTEGER PRIMARY KEY,
        name                TEXT NOT NULL UNIQUE,
        url                 TEXT NOT NULL,
        model               TEXT NOT NULL,
        ca_certificate      TEXT NOT NULL,
        headers             TEXT NOT NULL,
        body                TEXT NOT NULL,
        user_role           TEXT NOT NULL,
        model_role          TEXT NOT NULL,
        system_role         TEXT NOT NULL,
        system_prompt       TEXT NOT NULL,
        answer              TEXT NOT NULL,
        text_path           TEXT NOT NULL,
        stop_path           TEXT NOT NULL,
        cut_reasons         TEXT NOT NULL,
        refused_reasons     TEXT NOT NULL,
        error_path          TEXT NOT NULL,
        model_path          TEXT NOT NULL,
        input_tokens_path   TEXT NOT NULL,
        output_tokens_path  TEXT NOT NULL,
        done_marker         TEXT NOT NULL,
        key_origin          TEXT,
        message_template    TEXT NOT NULL DEFAULT '{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}'
    );
";

/// The tables of the documents of standards and of the test projects
/// (steps 16 to 22 and 25 of docs/todo.md). Times are seconds since 1970 (UTC).
const EVALUATION_SCHEMA: &str = "
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

    -- A test project: the map of a standard (evaluation/scheme.rs).
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
    -- separated by commas (evaluation/scheme.rs).
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

    -- found: whether the quote was found on its page (evaluation/quote.rs).
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

/// The column `message_template`, for a table `ai_connections` made
/// before it existed (the same day, during step 15 of docs/todo.md).
///
/// TODO (step 10 of docs/todo.md): remove before the release, with
/// [`REMOVED`].
const MESSAGE_TEMPLATE_COLUMN: &str = "ALTER TABLE ai_connections ADD COLUMN message_template TEXT NOT NULL \
    DEFAULT '{\"role\": \"{{role}}\", \"content\": \"{{text}}\"}'";

/// What earlier versions kept and this one no longer uses: the API
/// key of Anthropic and the chosen Claude model, replaced by the AI
/// connections (step 15 of docs/todo.md). Removed, the key
/// overwritten in the file, whenever a database is opened.
///
/// TODO (step 10 of docs/todo.md): remove before the release; no
/// database of a released version holds these.
const REMOVED: &str = "
    PRAGMA secure_delete = ON;
    DELETE FROM secrets WHERE name = 'anthropic.api_key';
    DELETE FROM settings WHERE name = 'ai.model';
";

/// The open database of one evaluator.
#[derive(Debug)]
pub struct UserDb {
    connection: Connection,
}

impl UserDb {
    /// Prepares an open connection: the tables that are missing.
    fn from(connection: Connection) -> Result<UserDb, DbError> {
        connection.execute_batch(SCHEMA)?;
        connection.execute_batch(EVALUATION_SCHEMA)?;
        connection.execute_batch(REMOVED)?;

        let has_template: bool = connection.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('ai_connections') WHERE name = 'message_template'",
            [],
            |row| row.get(0),
        )?;

        if !has_template {
            connection.execute_batch(MESSAGE_TEMPLATE_COLUMN)?;
        }

        Ok(UserDb { connection })
    }

    /// The value of a setting, or `None` when it was never stored.
    pub fn setting(&self, name: &str) -> Result<Option<String>, DbError> {
        Ok(self
            .connection
            .query_row("SELECT value FROM settings WHERE name = ?1", [name], |row| row.get(0))
            .optional()?)
    }

    /// Stores a setting, replacing an earlier value.
    pub fn set_setting(&self, name: &str, value: &str) -> Result<(), DbError> {
        self.connection.execute(
            "INSERT INTO settings (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = excluded.value",
            params![name, value],
        )?;

        Ok(())
    }

    /// The value of a secret, or `None` when none is stored. The
    /// copy given out is wiped when it is dropped.
    pub fn secret(&self, name: &str) -> Result<Option<Zeroizing<String>>, DbError> {
        Ok(self
            .connection
            .query_row("SELECT value FROM secrets WHERE name = ?1", [name], |row| row.get::<_, String>(0))
            .optional()?
            .map(Zeroizing::new))
    }

    /// Stores a secret, replacing an earlier value.
    pub fn set_secret(&self, name: &str, value: &str) -> Result<(), DbError> {
        self.connection.execute(
            "INSERT INTO secrets (name, value) VALUES (?1, ?2)
             ON CONFLICT (name) DO UPDATE SET value = excluded.value",
            params![name, value],
        )?;

        Ok(())
    }

    /// Removes a secret; a secret that is not stored is not an
    /// error.
    ///
    /// SQLite marks the space of a removed row as free but does not
    /// overwrite it at once; `secure_delete` makes it overwrite the
    /// value with zeros, so that the old secret is not left in the
    /// (encrypted) file.
    pub fn remove_secret(&self, name: &str) -> Result<(), DbError> {
        self.connection.execute_batch("PRAGMA secure_delete = ON")?;
        self.connection.execute("DELETE FROM secrets WHERE name = ?1", [name])?;

        Ok(())
    }

    /// The connection, for the modules that keep their own tables
    /// here (for example `ai/chats.rs`).
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

/// The path of the database of an account.
pub fn path(data_dir: &Path, user_id: i64) -> PathBuf {
    data_dir.join(DIRECTORY).join(format!("{user_id}.db"))
}

/// Creates the empty database of a new account.
pub fn create(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDb, DbError> {
    let directory = data_dir.join(DIRECTORY);

    // The folder is private to the user of the computer.
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)
        .map_err(|source| DbError::Io {
            path: directory.clone(),
            source,
        })?;

    UserDb::from(db::create(&path(data_dir, user_id), key)?)
}

/// Opens the database of an account with its key.
pub fn open(data_dir: &Path, user_id: i64, key: &DatabaseKey) -> Result<UserDb, DbError> {
    UserDb::from(db::open(&path(data_dir, user_id), key)?)
}

/// Removes the databases of all accounts; used when the program is
/// uninstalled. A missing folder is not an error.
pub fn remove_all(data_dir: &Path) -> Result<(), DbError> {
    let directory = data_dir.join(DIRECTORY);

    match fs::remove_dir_all(&directory) {
        Err(source) if source.kind() != io::ErrorKind::NotFound => Err(DbError::Io {
            path: directory,
            source,
        }),
        _ => Ok(()),
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "user_db_tests.rs"]
mod tests;
