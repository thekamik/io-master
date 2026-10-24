//! Forgetting the shared catalog of the requirements of the laboratory.

use crate::db::error::DbError;
use crate::db::user;
use crate::db::user::database::UserDb;

use super::read::{self, SETTING};

/// Forgets the shared catalog when it is `uuid` (the document was deleted);
/// the next requirement of the laboratory makes a new one.
///
/// Called by: `views/evaluation_plan/oscal_documents/file.rs` (`delete`).
pub fn catalog_if(db: &UserDb, uuid: &str) -> Result<(), DbError> {
    if read::catalog(db)?.as_deref() == Some(uuid) {
        user::delete::remove_setting(db, SETTING)?;
    }

    Ok(())
}
