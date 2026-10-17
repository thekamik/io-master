//! Changing what the user told about a document of a standard.

use crate::db::standards::update as change;
use crate::db::user::database::UserDb;

use super::model::{DocumentError, DocumentFields};

/// Changes what the user told about a document.
pub fn update(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DocumentError> {
    fields.check()?;

    Ok(change::document(db, id, fields)?)
}
