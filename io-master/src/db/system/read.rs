//! Reading the system database: the installation and the accounts.

use rusqlite::OptionalExtension;

use crate::db::error::DbError;
use crate::installation::installation::{ID_LENGTH, Installation, InstallationId, Role};

use super::database::SystemDb;
use super::rows::{UserRecord, user_from_row};

impl SystemDb {
    /// The record of the installation.
    pub fn installation(&self) -> Result<Installation, DbError> {
        let (id, role): (Vec<u8>, String) =
            self.connection()
                .query_row("SELECT id, role FROM installation", [], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let id: InstallationId =
            id.try_into().map_err(|_| DbError::Sql(format!("the identifier does not have {ID_LENGTH} bytes")))?;

        let role = Role::from_name(&role).ok_or_else(|| DbError::Sql(format!("unknown role '{role}'")))?;

        Ok(Installation { id, role })
    }

    /// The names of all accounts, the oldest first.
    pub fn user_names(&self) -> Result<Vec<String>, DbError> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT name FROM users ORDER BY id")?;

        let names = statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;

        Ok(names)
    }

    /// The account with the given name, letter case aside.
    pub fn user(&self, name: &str) -> Result<Option<UserRecord>, DbError> {
        Ok(self.connection().query_row("SELECT * FROM users WHERE name = ?1", [name], user_from_row).optional()?)
    }
}
