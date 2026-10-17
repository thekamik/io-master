//! Changing what the user told about a document of a standard.

use crate::db::standards::update as change;
use crate::db::user::database::UserDb;

use crate::model::standards::DocumentFields;
use crate::standards::error::DocumentError;

/// Changes what the user told about a document.
pub fn update(db: &UserDb, id: i64, fields: &DocumentFields) -> Result<bool, DocumentError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    fields.check()?;

    Ok(change::document(db, id, fields)?)
}
