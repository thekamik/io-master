//! The keys of the database of a user (`UserKeys`), in the table
//! `user_keys` of the system database, which is not encrypted: create,
//! read, update; never deleted (user, 2026-10-28). Nothing of it is
//! secret: the two copies of the key are useless without the password or
//! the recovery code (`panels/logic/program_start/keys.rs`, `panels/logic/program_start/recovery.rs`).
//!
//! ```text
//! user_keys   user (one row each); the copy wrapped with the password: key_salt, key_memory_kib,
//!             key_iterations, key_nonce, key_ciphertext, key_tag; the copy wrapped with the
//!             recovery code: recovery_salt, recovery_nonce, recovery_ciphertext, recovery_tag
//! ```

use rusqlite::named_params;

use crate::db::error::DbError;
use crate::db::system_database::SystemDatabase;
use crate::model::user::user_keys::{RecoveryWrappedKey, UserKeys, WrappedKey};

/// The table, made when it is missing (`system_database.rs`), after the
/// users.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS user_keys (
        user                INTEGER PRIMARY KEY REFERENCES users(id),
        key_salt            BLOB    NOT NULL,
        key_memory_kib      INTEGER NOT NULL,
        key_iterations      INTEGER NOT NULL,
        key_nonce           BLOB    NOT NULL,
        key_ciphertext      BLOB    NOT NULL,
        key_tag             BLOB    NOT NULL,
        recovery_salt       BLOB    NOT NULL,
        recovery_nonce      BLOB    NOT NULL,
        recovery_ciphertext BLOB    NOT NULL,
        recovery_tag        BLOB    NOT NULL
    );
";

/// Refuses a copy of a key with an empty part: the program never makes one.
///
/// Called by: [`create_user_keys`], [`update_user_keys`].
fn check_keys(keys: &UserKeys) -> Result<(), DbError> {
    let password = &keys.password;
    let recovery = &keys.recovery;

    ensure!(
        !password.salt.is_empty() && !password.nonce.is_empty() && !password.ciphertext.is_empty(),
        DbError::Inconsistent("a copy of a key with an empty part".to_owned())
    );
    ensure!(
        !recovery.salt.is_empty() && !recovery.nonce.is_empty() && !recovery.ciphertext.is_empty(),
        DbError::Inconsistent("a copy of a key with an empty part".to_owned())
    );

    Ok(())
}

/// Stores the keys of a user that has none.
///
/// Called by: `panels/logic/program_start/access.rs` (`create_account`).
/// Calls: [`check_keys`].
pub fn create_user_keys(system: &SystemDatabase, keys: &UserKeys) -> Result<(), DbError> {
    check_keys(keys)?;

    system.connection().execute(
        "INSERT INTO user_keys (user, key_salt, key_memory_kib, key_iterations, key_nonce, key_ciphertext, key_tag,
                                recovery_salt, recovery_nonce, recovery_ciphertext, recovery_tag)
         VALUES (:user, :key_salt, :key_memory_kib, :key_iterations, :key_nonce, :key_ciphertext, :key_tag,
                 :recovery_salt, :recovery_nonce, :recovery_ciphertext, :recovery_tag)",
        named_params! {
            ":user": keys.user,
            ":key_salt": keys.password.salt,
            ":key_memory_kib": keys.password.memory_kib,
            ":key_iterations": keys.password.iterations,
            ":key_nonce": keys.password.nonce,
            ":key_ciphertext": keys.password.ciphertext,
            ":key_tag": keys.password.tag,
            ":recovery_salt": keys.recovery.salt,
            ":recovery_nonce": keys.recovery.nonce,
            ":recovery_ciphertext": keys.recovery.ciphertext,
            ":recovery_tag": keys.recovery.tag,
        },
    )?;

    Ok(())
}

/// Reads the keys of the user `user`.
///
/// Called by: `panels/logic/program_start/access.rs` (`log_in`).
pub fn read_user_keys(system: &SystemDatabase, user: i64) -> Result<UserKeys, DbError> {
    let connection = system.connection();
    let mut statement = connection.prepare(
        "SELECT key_salt, key_memory_kib, key_iterations, key_nonce, key_ciphertext, key_tag,
                recovery_salt, recovery_nonce, recovery_ciphertext, recovery_tag
         FROM user_keys WHERE user = :user",
    )?;
    let mut rows = statement.query(named_params! { ":user": user })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "user_keys", id: user });
    };

    Ok(UserKeys {
        user,
        password: WrappedKey {
            salt: row.get("key_salt")?,
            memory_kib: row.get("key_memory_kib")?,
            iterations: row.get("key_iterations")?,
            nonce: row.get("key_nonce")?,
            ciphertext: row.get("key_ciphertext")?,
            tag: row.get("key_tag")?,
        },
        recovery: RecoveryWrappedKey {
            salt: row.get("recovery_salt")?,
            nonce: row.get("recovery_nonce")?,
            ciphertext: row.get("recovery_ciphertext")?,
            tag: row.get("recovery_tag")?,
        },
    })
}

/// Writes both copies of the key of a user anew, in one statement: after a
/// change of the password or a new recovery code, the old ones stop working
/// together.
///
/// Called by: nothing yet (the change of the password, the recovery).
/// Calls: [`check_keys`].
pub fn update_user_keys(system: &SystemDatabase, keys: &UserKeys) -> Result<(), DbError> {
    check_keys(keys)?;

    let changed = system.connection().execute(
        "UPDATE user_keys SET key_salt = :key_salt, key_memory_kib = :key_memory_kib,
                key_iterations = :key_iterations, key_nonce = :key_nonce, key_ciphertext = :key_ciphertext,
                key_tag = :key_tag, recovery_salt = :recovery_salt, recovery_nonce = :recovery_nonce,
                recovery_ciphertext = :recovery_ciphertext, recovery_tag = :recovery_tag
         WHERE user = :user",
        named_params! {
            ":user": keys.user,
            ":key_salt": keys.password.salt,
            ":key_memory_kib": keys.password.memory_kib,
            ":key_iterations": keys.password.iterations,
            ":key_nonce": keys.password.nonce,
            ":key_ciphertext": keys.password.ciphertext,
            ":key_tag": keys.password.tag,
            ":recovery_salt": keys.recovery.salt,
            ":recovery_nonce": keys.recovery.nonce,
            ":recovery_ciphertext": keys.recovery.ciphertext,
            ":recovery_tag": keys.recovery.tag,
        },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "user_keys", id: keys.user });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the keys of a user: stored, read, replaced together.

    use super::*;

    use crate::db::system_database::create;
    use crate::db::user::user_data::create_user;
    use crate::model::user::user_data::User;
    use crate::test_support::TempDir;

    fn keys(user: i64, byte: u8) -> UserKeys {
        UserKeys {
            user,
            password: WrappedKey {
                salt: vec![byte; 16],
                memory_kib: 65_536,
                iterations: 3,
                nonce: vec![byte; 12],
                ciphertext: vec![byte; 32],
                tag: vec![byte; 16],
            },
            recovery: RecoveryWrappedKey {
                salt: vec![byte; 16],
                nonce: vec![byte; 12],
                ciphertext: vec![byte; 32],
                tag: vec![byte; 16],
            },
        }
    }

    #[test]
    fn the_keys_of_a_user_are_stored_read_and_replaced() {
        let directory = TempDir::new();
        let system = create(directory.path()).unwrap();
        let user = create_user(&system, &User { id: 0, name: "Kamil".to_owned(), created: 1 }).unwrap();

        create_user_keys(&system, &keys(user, 1)).unwrap();

        assert_eq!(read_user_keys(&system, user).unwrap(), keys(user, 1));

        update_user_keys(&system, &keys(user, 2)).unwrap();

        assert_eq!(read_user_keys(&system, user).unwrap(), keys(user, 2));
    }

    #[test]
    fn keys_of_no_user_and_empty_keys_are_refused() {
        let directory = TempDir::new();
        let system = create(directory.path()).unwrap();
        let mut empty = keys(1, 1);

        empty.password.salt.clear();

        assert!(matches!(create_user_keys(&system, &keys(1, 1)), Err(DbError::Sql(_))));
        assert!(matches!(create_user_keys(&system, &empty), Err(DbError::Inconsistent(_))));
        assert!(matches!(read_user_keys(&system, 1), Err(DbError::NoRow { .. })));
        assert!(matches!(update_user_keys(&system, &keys(1, 1)), Err(DbError::NoRow { .. })));
    }
}
