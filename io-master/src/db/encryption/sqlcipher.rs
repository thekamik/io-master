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
//! **The parameters in effect** (the defaults of SQLCipher 4, except the
//! three of the memory, which `open()` sets; checked by the test
//! `the_parameters_of_the_encryption_are_those_described`):
//!
//! | Parameter | Value |
//! |---|---|
//! | SQLCipher | 4.x community, built into the program (`rusqlite`, feature `bundled-sqlcipher`) |
//! | cryptographic provider | OpenSSL of the system |
//! | cipher | AES-256 in CBC mode, a random IV per page |
//! | integrity | HMAC-SHA512 per page (`cipher_use_hmac = 1`) |
//! | page | 4096 bytes; no part of the header left plain |
//! | derivation of the key | none: the key is raw bytes (PBKDF2-HMAC-SHA512, 256000 rounds, only for a passphrase) |
//! | memory security of SQLCipher | on (`cipher_memory_security = 1`): the memory it frees is wiped, its keys are locked in RAM (never in the swap) |
//! | temporary data of SQLite | in memory only (`temp_store = 2`), never in a file beside the database |
//! | cache of pages | 2 MiB (`cache_size = -2048`): only the pages a query reads are decrypted, and few are kept |
//!
//! The memory: decrypted data is only in RAM, and only what the queries of
//! the moment read (step 47 of docs/history.md, user 2026-10-23); the file
//! on the disk is always encrypted, so a cut of the power leaves nothing
//! readable.
//!
//! Where the key comes from: `keys.rs` (the key of the database of an
//! evaluator, random, stored only wrapped) and `database_key.rs`.
//!
//! The databases of the program (`docs/architecture.md`,
//! section 5.3):
//!
//! ```text
//! db/system/   system.db       the installation and the accounts;
//!                              NOT encrypted (see create_plain())
//! db/user/     users/<id>.db   the data of one evaluator; encrypted
//!                              (and the areas db/ai/, db/standards/,
//!                              and the further areas of db/,
//!                              whose tables live in it)
//! ```
//!
//! This file is the only place where a database file is opened or
//! created, so the choice "encrypted or not" is made here and nowhere
//! else.

use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::db_old::error::DbError;

use super::database_key::DatabaseKey;

/// The version of the layout of the databases, stored in every
/// new one (`PRAGMA user_version`). A later version of the program
/// that changes the tables reads it to know what to convert.
pub const SCHEMA_VERSION: i64 = 1;

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
            return Err(DbError::Io { path: path.to_owned(), source });
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
/// holds nothing secret (`db/system/`). Its file is readable only
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

    // Before the key: SQLCipher wipes the memory it frees and locks its
    // keys in RAM from the start.
    connection.execute_batch("PRAGMA cipher_memory_security = ON")?;

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

    // Only the minimum decrypted in RAM: the temporary data of SQLite (a
    // sort, an index being built) in memory, never in a file, and a small
    // cache of the pages read.
    connection.execute_batch("PRAGMA temp_store = MEMORY; PRAGMA cache_size = -2048")?;

    // Foreign keys are checked by SQLite only when asked to.
    connection.execute_batch("PRAGMA foreign_keys = ON")?;

    Ok(connection)
}

#[cfg(test)]
mod tests {
    //! Unit tests of `sqlcipher.rs`.

    use super::*;

    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use crate::security::random::fill_random;
    use crate::test_support::TempDir;

    /// A random key.
    fn key() -> DatabaseKey {
        DatabaseKey::fill_with(|bytes| fill_random(bytes)).unwrap()
    }

    #[test]
    fn a_database_opens_again_with_its_key() {
        let directory = TempDir::new();
        let path = directory.path().join("test.db");
        let key = key();

        create(&path, &key).unwrap().execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('kept');").unwrap();

        let value: String = open(&path, &key).unwrap().query_row("SELECT v FROM t", [], |row| row.get(0)).unwrap();

        assert_eq!(value, "kept");
    }

    #[test]
    fn the_file_is_encrypted_and_private() {
        let directory = TempDir::new();
        let path = directory.path().join("test.db");

        create(&path, &key())
            .unwrap()
            .execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('a readable secret');")
            .unwrap();

        let bytes = fs::read(&path).unwrap();

        // A plain SQLite file starts with this text; an encrypted one
        // does not, and the stored text is not readable either.
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(!bytes.windows(17).any(|window| window == b"a readable secret"));

        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn another_key_does_not_open_the_database() {
        let directory = TempDir::new();
        let path = directory.path().join("test.db");

        create(&path, &key()).unwrap().execute_batch("CREATE TABLE t (v)").unwrap();

        assert!(matches!(open(&path, &key()), Err(DbError::WrongKey(_))));
    }

    #[test]
    fn an_existing_database_is_never_created_again() {
        let directory = TempDir::new();
        let path = directory.path().join("test.db");

        create(&path, &key()).unwrap();

        assert!(matches!(create(&path, &key()), Err(DbError::Exists(_))));
        assert!(matches!(open(&directory.path().join("missing.db"), &key()), Err(DbError::NotFound(_))));
    }

    #[test]
    fn a_new_empty_database_opens_only_with_its_key() {
        let directory = TempDir::new();
        let path = directory.path().join("empty.db");

        // No table is created: the database stays as create() made it.
        create(&path, &key()).unwrap();

        assert!(fs::metadata(&path).unwrap().len() > 0);
        assert!(matches!(open(&path, &key()), Err(DbError::WrongKey(_))));
    }

    #[test]
    fn every_database_knows_its_schema_version() {
        let directory = TempDir::new();
        let path = directory.path().join("versioned.db");
        let key = key();

        create(&path, &key).unwrap();

        let version: i64 = open(&path, &key).unwrap().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    /// The value of a `PRAGMA` of SQLCipher.
    fn pragma(connection: &Connection, name: &str) -> String {
        connection.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0)).unwrap()
    }

    /// The value of a `PRAGMA` that SQLite gives as a number.
    fn number(connection: &Connection, name: &str) -> i64 {
        connection.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0)).unwrap()
    }

    /// The parameters of the encryption in effect, as the head of
    /// `sqlcipher.rs` describes them. The program does not set them: they
    /// are the defaults of SQLCipher 4. A newer SQLCipher that changed a
    /// default fails here instead of changing the encryption unnoticed.
    #[test]
    fn the_parameters_of_the_encryption_are_those_described() {
        let directory = TempDir::new();
        let connection = create(&directory.path().join("p.db"), &key()).unwrap();

        assert!(pragma(&connection, "cipher_version").starts_with("4."), "{}", pragma(&connection, "cipher_version"));
        assert_eq!(pragma(&connection, "cipher_provider"), "openssl");
        assert_eq!(pragma(&connection, "cipher_page_size"), "4096");
        assert_eq!(pragma(&connection, "cipher_use_hmac"), "1");
        assert_eq!(pragma(&connection, "cipher_hmac_algorithm"), "HMAC_SHA512");
        assert_eq!(pragma(&connection, "cipher_plaintext_header_size"), "0");

        // Not used: the key is given as raw bytes, so SQLCipher derives
        // nothing from it (see open()).
        assert_eq!(pragma(&connection, "cipher_kdf_algorithm"), "PBKDF2_HMAC_SHA512");
        assert_eq!(pragma(&connection, "kdf_iter"), "256000");

        // The memory (step 47): wiped and locked, the temporary data in
        // memory, a small cache.
        assert_eq!(pragma(&connection, "cipher_memory_security"), "1");
        assert_eq!(number(&connection, "temp_store"), 2);
        assert_eq!(number(&connection, "cache_size"), -2048);
    }
}
