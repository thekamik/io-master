//! The user of the installation (`User`), in the table `users` of the
//! system database, which is not encrypted: create, read, update; never
//! deleted (user, 2026-10-28). One Linux account holds one installation
//! with one user.
//!
//! ```text
//! users   id, name (unique whatever its letter case), created
//! ```

#![allow(dead_code, reason = "the new database (2026-10-28), not used by the views yet, only by its tests")]

use rusqlite::{Row, named_params};

use crate::db::error::DbError;
use crate::db::system_database::SystemDatabase;
use crate::model::user::user_data::User;

/// The most users of an installation: one (`docs/architecture.md`, section 3).
pub const MAX_USERS: usize = 1;

/// The longest name of a user, in characters.
pub const MAX_NAME: usize = 64;

/// The table, made when it is missing (`system_database.rs`). Names that
/// differ only in their letter case are one name: 'Anna' and 'anna' would
/// be too easy to mix up.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS users (
        id      INTEGER PRIMARY KEY,
        name    TEXT NOT NULL UNIQUE COLLATE NOCASE,
        created INTEGER NOT NULL
    );
";

/// Refuses a name that is empty, over its limit, or holds a sign that is no
/// letter, digit, space, dot, dash or underscore.
///
/// Called by: [`create_user`], [`update_user`].
fn check_name(name: &str) -> Result<(), DbError> {
    ensure!(!name.trim().is_empty(), DbError::Inconsistent("a user without a name".to_owned()));
    ensure!(name.chars().count() <= MAX_NAME, DbError::Inconsistent(format!("a name over {MAX_NAME} characters")));

    for character in name.chars() {
        ensure!(
            character.is_alphanumeric() || " .-_".contains(character),
            DbError::Inconsistent(format!("the sign '{character}' in a name"))
        );
    }

    Ok(())
}

/// Stores the user of the installation and gives its id; refused when the
/// installation has its user already. A new object has no id yet (`id`
/// 0): the database gives it.
///
/// Called by: nothing yet (the setup rewritten on `db/`).
/// Calls: [`check_name`].
pub fn create_user(system: &SystemDatabase, user: &User) -> Result<i64, DbError> {
    ensure!(user.id == 0, DbError::Inconsistent("a new user has an id already".to_owned()));
    check_name(&user.name)?;

    let connection = system.connection();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;

    ensure!(
        usize::try_from(count).map_err(|error| DbError::Inconsistent(error.to_string()))? < MAX_USERS,
        DbError::Inconsistent("the installation has its user already".to_owned())
    );

    connection.execute(
        "INSERT INTO users (name, created) VALUES (:name, :created)",
        named_params! { ":name": user.name, ":created": user.created },
    )?;

    Ok(connection.last_insert_rowid())
}

/// The model of a row of `users`.
///
/// Called by: [`read_user`], [`read_users`].
fn user_of(row: &Row<'_>) -> Result<User, DbError> {
    Ok(User { id: row.get("id")?, name: row.get("name")?, created: row.get("created")? })
}

/// Reads the user `id`.
///
/// Called by: nothing yet (the login rewritten on `db/`).
/// Calls: [`user_of`].
pub fn read_user(system: &SystemDatabase, id: i64) -> Result<User, DbError> {
    let connection = system.connection();
    let mut statement = connection.prepare("SELECT id, name, created FROM users WHERE id = :id")?;
    let mut rows = statement.query(named_params! { ":id": id })?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "users", id });
    };

    user_of(row)
}

/// Reads the users of the installation: none before the setup, one after.
///
/// Called by: nothing yet (the login rewritten on `db/`).
/// Calls: [`user_of`].
pub fn read_users(system: &SystemDatabase) -> Result<Vec<User>, DbError> {
    let mut statement = system.connection().prepare("SELECT id, name, created FROM users ORDER BY id")?;
    let mut rows = statement.query([])?;
    let mut users = Vec::with_capacity(MAX_USERS);

    for _ in 0..=MAX_USERS {
        let Some(row) = rows.next()? else {
            return Ok(users);
        };

        users.push(user_of(row)?);
    }

    Err(DbError::Inconsistent(format!("more than {MAX_USERS} users in the database")))
}

/// Writes the user `user.id` anew: a new name.
///
/// Called by: nothing yet (the views rewritten on `db/`).
/// Calls: [`check_name`].
pub fn update_user(system: &SystemDatabase, user: &User) -> Result<(), DbError> {
    check_name(&user.name)?;

    let changed = system.connection().execute(
        "UPDATE users SET name = :name, created = :created WHERE id = :id",
        named_params! { ":id": user.id, ":name": user.name, ":created": user.created },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "users", id: user.id });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the user: one per installation, read, renamed, its name
    //! checked.

    use super::*;

    use crate::db::system_database::create;
    use crate::test_support::TempDir;

    fn user(name: &str) -> User {
        User { id: 0, name: name.to_owned(), created: 1_793_000_000 }
    }

    #[test]
    fn the_one_user_is_stored_read_and_renamed() {
        let directory = TempDir::new();
        let system = create(directory.path()).unwrap();
        let id = create_user(&system, &user("Kamil")).unwrap();
        let mut stored = read_user(&system, id).unwrap();

        assert_eq!(stored, User { id, ..user("Kamil") });
        assert!(matches!(create_user(&system, &user("Anna")), Err(DbError::Inconsistent(_))));

        stored.name = "Kamil K.".to_owned();
        update_user(&system, &stored).unwrap();

        assert_eq!(read_users(&system).unwrap(), vec![stored]);
    }

    #[test]
    fn a_name_that_is_no_name_is_refused() {
        let directory = TempDir::new();
        let system = create(directory.path()).unwrap();

        for name in ["", "   ", "a/b", "x\u{0}y", &"a".repeat(MAX_NAME + 1)] {
            assert!(matches!(create_user(&system, &user(name)), Err(DbError::Inconsistent(_))), "{name:?}");
        }

        assert!(read_users(&system).unwrap().is_empty());
        assert!(matches!(read_user(&system, 1), Err(DbError::NoRow { .. })));
    }
}
