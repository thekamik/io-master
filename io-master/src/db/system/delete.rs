//! Removing rows of the system database: an account. (Removing the
//! whole file is `SystemDb::remove` in `database.rs`.)

use crate::db::error::DbError;

use super::database::SystemDb;

/// Removes an account whose database could not be made, so that
/// its name is free again.
pub fn remove_user(db: &SystemDb, id: i64) -> Result<(), DbError> {
    db.connection().execute("DELETE FROM users WHERE id = ?1", [id])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of removing an account.

    use super::*;

    use crate::db::system::{create, read};
    use crate::test_support::TempDir;
    use crate::test_support::db::{system_db, wrapped_key};

    #[test]
    fn a_removed_account_is_found_no_more() {
        let directory = TempDir::new();
        let db = system_db(&directory);
        let id = create::add_user(&db, "anna", &wrapped_key()).unwrap();

        remove_user(&db, id).unwrap();

        assert_eq!(read::user(&db, "anna").unwrap(), None);
    }
}
