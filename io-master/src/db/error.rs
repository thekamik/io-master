//! The error of every function of `db/`: why a database could not be
//! used. The functions return the models of `model/` and this error;
//! the window tells its kinds apart.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why a database could not be used.
#[derive(Debug)]
pub enum DbError {
    /// The database does not exist.
    NotFound(PathBuf),
    /// A database exists where a new one was to be made; an existing
    /// database is never overwritten.
    Exists(PathBuf),
    /// The key does not open the database, or the file is not a
    /// database of this program.
    WrongKey(PathBuf),
    /// SQLite reported an error; holds its message.
    Sql(String),
    /// A value in the database that the program never writes (a verdict,
    /// a state): the file was changed outside the program. Never replaced
    /// in silence by a value that looks right.
    Corrupt { table: &'static str, column: &'static str, value: String },
    /// The row to read or change does not exist.
    NoRow { table: &'static str, id: i64 },
    /// What was given to be stored does not hold together (a work unit
    /// that depends on itself, a quote of a document it does not use, a
    /// list longer than its limit); a mistake of the caller.
    Inconsistent(String),
    /// A file or a folder could not be used.
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
            DbError::Corrupt { table, column, value } => write!(
                formatter,
                "database: the column {column} of {table} holds '{value}', which this program never writes"
            ),
            DbError::NoRow { table, id } => write!(formatter, "database: no row {id} in {table}"),
            DbError::Inconsistent(reason) => write!(formatter, "database: {reason}"),
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
