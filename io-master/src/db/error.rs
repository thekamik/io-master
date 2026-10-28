//! The error of every function of the database layer (`db/`).
//!
//! The functions of `db/` return plain Rust types and this error;
//! the interface tells its kinds apart (`app/error.rs`).

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::model_old::oscal::NotOscal;

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
    /// A value in the database that the program never writes (a kind,
    /// a state, a name of a role): the file was changed outside the
    /// program, or by a newer version. Never replaced in silence by a
    /// value that looks right.
    Corrupt(Corrupt),
    /// The row to change does not exist.
    NoRow { table: &'static str, id: i64 },
    /// A document of OSCAL that breaks the schema of NIST: never
    /// stored.
    NotOscal(NotOscal),
    /// What was given to be stored does not hold together (a tile of a
    /// scheme that is not stored with it); a mistake of the caller.
    Inconsistent(String),
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
            DbError::Corrupt(corrupt) => write!(formatter, "{corrupt}"),
            DbError::NoRow { table, id } => write!(formatter, "database: no row {id} in {table}"),
            DbError::Inconsistent(reason) => write!(formatter, "database: {reason}"),
            DbError::NotOscal(error) => write!(formatter, "{error}"),
            DbError::Io { path, source } => write!(formatter, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    /// A [`Corrupt`] value found while a row was read comes through
    /// rusqlite as the cause of a failed conversion ([`corrupt`]); it
    /// is taken out again here, so that it keeps its kind.
    fn from(error: rusqlite::Error) -> DbError {
        if let rusqlite::Error::FromSqlConversionFailure(_, _, cause) = &error
            && let Some(corrupt) = cause.downcast_ref::<Corrupt>()
        {
            return DbError::Corrupt(corrupt.clone());
        }

        DbError::Sql(error.to_string())
    }
}

/// A value the program never writes: where it is and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corrupt {
    pub table: &'static str,
    pub column: &'static str,
    pub value: String,
}

impl fmt::Display for Corrupt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "database: the column {} of {} holds '{}', which this program never writes",
            self.column, self.table, self.value
        )
    }
}

impl std::error::Error for Corrupt {}

/// The error a function that reads a row (`rows.rs`) gives for a
/// value the program never writes. It travels through rusqlite and
/// becomes [`DbError::Corrupt`].
pub(in crate::db) fn corrupt(table: &'static str, column: &'static str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(Corrupt { table, column, value: value.to_owned() }),
    )
}

/// The value of `column` of `table` named `name`, read by `from_name`;
/// a name the program never writes is [`corrupt`].
pub(in crate::db) fn named<T>(
    table: &'static str,
    column: &'static str,
    name: &str,
    from_name: impl FnOnce(&str) -> Option<T>,
) -> rusqlite::Result<T> {
    from_name(name).ok_or_else(|| corrupt(table, column, name))
}
