//! The data of the program: the installation (`Installation`), in the table
//! `installation` of the system database, which is not encrypted, and the
//! settings of the user (`Settings`), in the table `settings` of the
//! encrypted database of the user. Never deleted (user, 2026-10-28).
//!
//! The installation is made once, at the first start, and never changes:
//! **create and read only**. The settings are one row: create, read,
//! update.
//!
//! ```text
//! installation   id (16 random bytes), created          (system database, one row)
//! settings       active_integration                     (database of the user, one row)
//! ```

#![allow(dead_code, reason = "the new database (2026-10-28), not used by the views yet, only by its tests")]

use rusqlite::named_params;

use crate::db::database::Database;
use crate::db::error::DbError;
use crate::db::system_database::SystemDatabase;
use crate::model::user::program_data::{Installation, Settings};

/// The table of the installation, made when it is missing
/// (`system_database.rs`).
pub const SYSTEM_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS installation (
        one     INTEGER PRIMARY KEY CHECK (one = 1),
        id      BLOB NOT NULL CHECK (length(id) = 16),
        created INTEGER NOT NULL
    );
";

/// The table of the settings, made when it is missing (`database.rs`),
/// after the integrations.
pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS settings (
        one                INTEGER PRIMARY KEY CHECK (one = 1),
        active_integration INTEGER REFERENCES ai_integrations(id)
    );
";

/// Stores the installation at the first start; refused when it is stored
/// already (the one row of its table).
///
/// Called by: nothing yet (the setup rewritten on `db/`).
pub fn create_installation(system: &SystemDatabase, installation: &Installation) -> Result<(), DbError> {
    ensure!(installation.id != [0; 16], DbError::Inconsistent("an installation without its random id".to_owned()));
    ensure!(installation.created > 0, DbError::Inconsistent("an installation without its time".to_owned()));

    system.connection().execute(
        "INSERT INTO installation (one, id, created) VALUES (1, :id, :created)",
        named_params! { ":id": installation.id, ":created": installation.created },
    )?;

    Ok(())
}

/// Reads the installation; `None` before the first start has stored it.
///
/// Called by: nothing yet (the start rewritten on `db/`).
pub fn read_installation(system: &SystemDatabase) -> Result<Option<Installation>, DbError> {
    let connection = system.connection();
    let mut statement = connection.prepare("SELECT id, created FROM installation WHERE one = 1")?;
    let mut rows = statement.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let id: Vec<u8> = row.get("id")?;
    let id: [u8; 16] = id.try_into().map_err(|_| DbError::Corrupt {
        table: "installation",
        column: "id",
        value: "not 16 bytes".to_owned(),
    })?;

    Ok(Some(Installation { id, created: row.get("created")? }))
}

/// Stores the settings of a new database of the user; refused when they are
/// stored already (the one row of its table).
///
/// Called by: nothing yet (the setup rewritten on `db/`).
pub fn create_settings(db: &Database, settings: &Settings) -> Result<(), DbError> {
    db.connection().execute(
        "INSERT INTO settings (one, active_integration) VALUES (1, :active_integration)",
        named_params! { ":active_integration": settings.active_integration },
    )?;

    Ok(())
}

/// Reads the settings of the user.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn read_settings(db: &Database) -> Result<Settings, DbError> {
    let connection = db.connection();
    let mut statement = connection.prepare("SELECT active_integration FROM settings WHERE one = 1")?;
    let mut rows = statement.query([])?;
    let Some(row) = rows.next()? else {
        return Err(DbError::NoRow { table: "settings", id: 1 });
    };

    Ok(Settings { active_integration: row.get("active_integration")? })
}

/// Writes the settings of the user anew.
///
/// Called by: nothing yet (the views rewritten on `db/`).
pub fn update_settings(db: &Database, settings: &Settings) -> Result<(), DbError> {
    let changed = db.connection().execute(
        "UPDATE settings SET active_integration = :active_integration WHERE one = 1",
        named_params! { ":active_integration": settings.active_integration },
    )?;

    ensure!(changed == 1, DbError::NoRow { table: "settings", id: 1 });

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the data of the program: the installation stored once, the
    //! settings stored, read and changed.

    use super::*;

    use crate::db::ai::integration::create_integration;
    use crate::db::system_database::create;
    use crate::test_support::TempDir;
    use crate::test_support::database::{database, integration};

    #[test]
    fn the_installation_is_stored_once_and_read() {
        let directory = TempDir::new();
        let system = create(directory.path()).unwrap();
        let installation = Installation { id: [7; 16], created: 1_793_000_000 };

        assert!(read_installation(&system).unwrap().is_none());

        create_installation(&system, &installation).unwrap();

        assert_eq!(read_installation(&system).unwrap(), Some(installation.clone()));
        assert!(matches!(create_installation(&system, &installation), Err(DbError::Sql(_))));
        assert!(matches!(
            create_installation(&system, &Installation { id: [0; 16], created: 1 }),
            Err(DbError::Inconsistent(_))
        ));
    }

    #[test]
    fn the_settings_are_stored_read_and_changed() {
        let (_directory, db) = database();

        assert!(matches!(read_settings(&db), Err(DbError::NoRow { .. })));
        assert!(matches!(update_settings(&db, &Settings { active_integration: None }), Err(DbError::NoRow { .. })));

        create_settings(&db, &Settings { active_integration: None }).unwrap();

        let integration = create_integration(&db, &integration()).unwrap();

        update_settings(&db, &Settings { active_integration: Some(integration) }).unwrap();

        assert_eq!(read_settings(&db).unwrap(), Settings { active_integration: Some(integration) });
        assert!(matches!(create_settings(&db, &Settings { active_integration: None }), Err(DbError::Sql(_))));
    }
}
