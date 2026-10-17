//! Removing rows of the system database: an account. (Removing the
//! whole file is `SystemDb::remove` in `database.rs`.)

use crate::db::error::DbError;

use super::database::SystemDb;

impl SystemDb {
    /// Removes an account whose database could not be made, so that
    /// its name is free again.
    pub fn remove_user(&self, id: i64) -> Result<(), DbError> {
        self.connection().execute("DELETE FROM users WHERE id = ?1", [id])?;

        Ok(())
    }
}
