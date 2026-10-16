//! The system database: `system.db` in the data directory.
//!
//! It is the record of the installation (`docs/architecture.md`,
//! sections 3.2 and 5.3): which installation this is, its role,
//! and its accounts. Its existence decides whether this is the
//! first start.
//!
//! **It is not encrypted.** The program opens it at every start,
//! before anyone has given a password, so no key could open it.
//! It therefore holds nothing secret: names, a role, and the keys
//! of the databases of the evaluators only in encrypted form, which
//! are useless without the passwords (`keys.rs`). The file is
//! readable only by the user.
//!
//! TODO (step 8 of docs/todo.md): with the optional TPM factor it
//! can be encrypted with a key from the secret of the device.
//!
//! ```text
//! installation   one row: identifier, role, time of the first start
//! users          one row per account: name, time it was created,
//!                and the encrypted key of its database (keys.rs)
//! ```
//!
//! Every account has its own database (`user_db.rs`) under
//! `users/<id>.db`; this database holds only what is needed to open
//! it.
//!
//! The connection is kept behind a lock, so that the commands of
//! the interface, which may run at the same time, use it one after
//! another.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::installation::installation::{ID_LENGTH, Installation, InstallationId, Role};

use super::db::{self, DbError};
use super::keys::WrappedKey;

/// Name of the file of the system database.
pub const FILE_NAME: &str = "system.db";

/// The tables, created with the database.
const SCHEMA: &str = "
    CREATE TABLE installation (
        id          BLOB    NOT NULL,
        role        TEXT    NOT NULL,
        created     INTEGER NOT NULL
    );

    CREATE TABLE users (
        id              INTEGER PRIMARY KEY,
        -- Names are unique whatever their letter case: 'Anna'
        -- and 'anna' would be too easy to mix up.
        name            TEXT    NOT NULL UNIQUE COLLATE NOCASE,
        created         INTEGER NOT NULL,
        key_salt        BLOB    NOT NULL,
        key_memory_kib  INTEGER NOT NULL,
        key_iterations  INTEGER NOT NULL,
        -- 1 when the secret of the device (TPM) is a factor.
        key_device      INTEGER NOT NULL,
        key_nonce       BLOB    NOT NULL,
        key_ciphertext  BLOB    NOT NULL,
        key_tag         BLOB    NOT NULL
    );
";

/// An account as the system database knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub id: i64,
    pub name: String,
    /// The encrypted key of the database of the account.
    pub key: WrappedKey,
}

/// The open system database.
#[derive(Debug)]
pub struct SystemDb {
    connection: Mutex<Connection>,
}

/// The path of the system database in a data directory.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Seconds since 1970, the time stamps of the database.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| i64::try_from(time.as_secs()).unwrap_or(i64::MAX))
}

/// Reads one row of the `users` table.
fn user_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRecord> {
    Ok(UserRecord {
        id: row.get("id")?,
        name: row.get("name")?,
        key: WrappedKey {
            salt: row.get("key_salt")?,
            memory_kib: row.get("key_memory_kib")?,
            iterations: row.get("key_iterations")?,
            with_device_secret: row.get("key_device")?,
            nonce: row.get("key_nonce")?,
            ciphertext: row.get("key_ciphertext")?,
            tag: row.get("key_tag")?,
        },
    })
}

impl SystemDb {
    /// Creates the system database at the first start, with the
    /// record of the installation. Fails when one exists.
    pub fn create(data_dir: &Path, installation: &Installation) -> Result<SystemDb, DbError> {
        let connection = db::create_plain(&path(data_dir))?;

        connection.execute_batch(SCHEMA)?;
        connection.execute(
            "INSERT INTO installation (id, role, created) VALUES (?1, ?2, ?3)",
            params![installation.id.as_slice(), installation.role.name(), now()],
        )?;

        Ok(SystemDb {
            connection: Mutex::new(connection),
        })
    }

    /// Opens the system database of a data directory.
    pub fn open(data_dir: &Path) -> Result<SystemDb, DbError> {
        let connection = db::open_plain(&path(data_dir))?;

        Ok(SystemDb {
            connection: Mutex::new(connection),
        })
    }

    /// Removes the system database; used when the program is
    /// uninstalled. A missing file is not an error.
    pub fn remove(data_dir: &Path) -> Result<(), DbError> {
        let path = path(data_dir);

        match fs::remove_file(&path) {
            Err(source) if source.kind() != io::ErrorKind::NotFound => Err(DbError::Io { path, source }),
            _ => Ok(()),
        }
    }

    /// The connection, for one piece of work. A panic in another
    /// request while it held the lock does not leave the database
    /// half-changed (SQLite rolls back an unfinished transaction),
    /// so the lock is used as it is.
    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The record of the installation.
    pub fn installation(&self) -> Result<Installation, DbError> {
        let (id, role): (Vec<u8>, String) = self.connection().query_row(
            "SELECT id, role FROM installation",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let id: InstallationId = id
            .try_into()
            .map_err(|_| DbError::Sql(format!("the identifier does not have {ID_LENGTH} bytes")))?;

        let role = Role::from_name(&role).ok_or_else(|| DbError::Sql(format!("unknown role '{role}'")))?;

        Ok(Installation { id, role })
    }

    /// The names of all accounts, the oldest first.
    pub fn user_names(&self) -> Result<Vec<String>, DbError> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT name FROM users ORDER BY id")?;

        let names = statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        Ok(names)
    }

    /// The account with the given name, letter case aside.
    pub fn user(&self, name: &str) -> Result<Option<UserRecord>, DbError> {
        Ok(self
            .connection()
            .query_row("SELECT * FROM users WHERE name = ?1", [name], user_from_row)
            .optional()?)
    }

    /// Adds an account and returns its id. Fails when the name is
    /// taken.
    pub fn add_user(&self, name: &str, key: &WrappedKey) -> Result<i64, DbError> {
        let connection = self.connection();

        connection.execute(
            "INSERT INTO users (name, created, key_salt, key_memory_kib, key_iterations,
                                key_device, key_nonce, key_ciphertext, key_tag)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                name,
                now(),
                key.salt,
                key.memory_kib,
                key.iterations,
                key.with_device_secret,
                key.nonce,
                key.ciphertext,
                key.tag
            ],
        )?;

        Ok(connection.last_insert_rowid())
    }

    /// Stores a newly encrypted key of an account, after a change
    /// of its password.
    pub fn replace_user_key(&self, id: i64, key: &WrappedKey) -> Result<(), DbError> {
        let changed = self.connection().execute(
            "UPDATE users SET key_salt = ?2, key_memory_kib = ?3, key_iterations = ?4,
                              key_device = ?5, key_nonce = ?6, key_ciphertext = ?7, key_tag = ?8
             WHERE id = ?1",
            params![
                id,
                key.salt,
                key.memory_kib,
                key.iterations,
                key.with_device_secret,
                key.nonce,
                key.ciphertext,
                key.tag
            ],
        )?;

        if changed == 1 {
            Ok(())
        } else {
            Err(DbError::Sql(format!("no account {id}")))
        }
    }

    /// Removes an account whose database could not be made, so that
    /// its name is free again.
    pub fn remove_user(&self, id: i64) -> Result<(), DbError> {
        self.connection().execute("DELETE FROM users WHERE id = ?1", [id])?;

        Ok(())
    }
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "system_db_tests.rs"]
mod tests;
