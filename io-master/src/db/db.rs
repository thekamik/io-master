//! Opening and creating encrypted databases.
//!
//! Every database of the program is an SQLite file encrypted by
//! SQLCipher: the whole file, page by page, with AES-256. Without
//! the key the file is random bytes; it does not even reveal that
//! it is a database.
//!
//! The key is given to SQLCipher as raw bytes (`PRAGMA key =
//! "x'...'"`), so SQLCipher uses it as it is and skips its own
//! slow derivation of a key from a password: the keys of this
//! program are random or derived in `keys.rs` already.
//!
//! The databases of the program (`docs/architecture.md`,
//! section 5.3):
//!
//! ```text
//! system_db.rs   system.db       the installation and the accounts;
//!                                not encrypted (see create_plain())
//! user_db.rs     users/<id>.db   the data of one evaluator; encrypted
//! ```

use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use zeroize::Zeroizing;

use super::database_key::DatabaseKey;

/// The version of the layout of the databases, stored in every
/// new one (`PRAGMA user_version`). A later version of the program
/// that changes the tables reads it to know what to convert.
pub const SCHEMA_VERSION: i64 = 1;

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

/// Creates the file of a new database, readable only by the user,
/// so that SQLite never creates it with wider permissions. Fails
/// when the file exists already.
fn create_file(path: &Path) -> Result<(), DbError> {
    match OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(DbError::Exists(path.to_owned()));
        }
        Err(source) => {
            return Err(DbError::Io {
                path: path.to_owned(),
                source,
            });
        }
    }

    Ok(())
}

/// Creates a new, empty encrypted database. Fails when the file
/// exists already.
pub fn create(path: &Path, key: &DatabaseKey) -> Result<Connection, DbError> {
    create_file(path)?;

    let connection = open(path, key)?;

    // SQLCipher writes nothing until the first change, and an empty
    // file opens with any key. The version of the schema is written
    // at once, so the file holds its first encrypted page from the
    // start and only its key opens it.
    connection.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;

    Ok(connection)
}

/// Creates a new database that is **not** encrypted. Fails when the
/// file exists already.
///
/// Used only for the system database, which must open at program
/// start, before anyone has given a password, and which therefore
/// holds nothing secret (`system_db.rs`). Its file is readable only
/// by the user.
pub fn create_plain(path: &Path) -> Result<Connection, DbError> {
    create_file(path)?;

    let connection = open_plain(path)?;

    connection.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;

    Ok(connection)
}

/// Opens a database that is not encrypted. A file that is not such
/// a database (for example an encrypted one) is refused.
pub fn open_plain(path: &Path) -> Result<Connection, DbError> {
    if !path.exists() {
        return Err(DbError::NotFound(path.to_owned()));
    }

    let connection = Connection::open(path)?;

    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|_| DbError::WrongKey(path.to_owned()))?;

    connection.execute_batch("PRAGMA foreign_keys = ON")?;

    Ok(connection)
}

/// Opens an existing encrypted database with its key.
pub fn open(path: &Path, key: &DatabaseKey) -> Result<Connection, DbError> {
    if !path.exists() {
        return Err(DbError::NotFound(path.to_owned()));
    }

    let connection = Connection::open(path)?;

    // The statement with the key, written into memory that is
    // wiped afterwards: 64 hexadecimal digits of the raw key.
    let mut statement = Zeroizing::new(String::with_capacity(80));

    statement.push_str("PRAGMA key = \"x'");

    for byte in key.expose() {
        statement.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        statement.push(char::from(b"0123456789abcdef"[usize::from(byte & 0x0f)]));
    }

    statement.push_str("'\"");

    connection.execute_batch(&statement)?;

    // SQLCipher checks the key only when the file is first read.
    // A wrong key, or a file that is not a database, fails here.
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(|_| DbError::WrongKey(path.to_owned()))?;

    // Foreign keys are checked by SQLite only when asked to.
    connection.execute_batch("PRAGMA foreign_keys = ON")?;

    Ok(connection)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "db_tests.rs"]
mod tests;
