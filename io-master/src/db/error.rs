//! The error of every function of the database layer (`db/`).
//!
//! The functions of `db/` return plain Rust types and this error;
//! the interface tells its kinds apart (`tauri_ui/error.rs`).

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why a database could not be used.
#[derive(Debug)]
pub enum DbError {
    /// The database does not exist.
    NotFound(PathBuf),
    /// A database exists where a new one was to be created; an
    /// existing database is never overwritten.
    Exists(PathBuf),
    /// The key does not open the database, or the file is not
    /// a database of this program.
    WrongKey(PathBuf),
    /// SQLite reported an error; holds its message.
    Sql(String),
    /// A file or directory could not be used.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for DbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::NotFound(path) => write!(formatter, "{}: no such database", path.display()),
            DbError::Exists(path) => write!(formatter, "{}: the database exists already", path.display()),
            DbError::WrongKey(path) => {
                write!(formatter, "{}: the database cannot be opened with this key", path.display())
            }
            DbError::Sql(message) => write!(formatter, "database: {message}"),
            DbError::Io { path, source } => write!(formatter, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> DbError {
        DbError::Sql(error.to_string())
    }
}
