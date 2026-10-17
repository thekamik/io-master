//! Making a test project: written to the database at once; it is
//! opened for editing by `workspace.rs`.

use crate::db::test_projects::{create as store, read as stored};
use crate::db::user::database::UserDb;
use crate::standards;

use crate::model::test_projects::ProjectFields;
use crate::model::test_projects::rules::SchemeError;
use crate::test_projects::error::StoreError;

/// Checks the fields and the documents of a project, and its name
/// against the other projects (`id`: the project itself, when it is
/// changed).
pub(super) fn check(db: &UserDb, id: Option<i64>, fields: &ProjectFields) -> Result<(), StoreError> {
    fields.check()?;

    for document in &fields.documents {
        if standards::read::get(db, *document)?.is_none() {
            return Err(SchemeError::NotFound("document").into());
        }
    }

    let taken = stored::project_named(db, fields.name.trim())?;

    if taken.is_some_and(|other| Some(other) != id) {
        return Err(SchemeError::NameTaken.into());
    }

    Ok(())
}

/// Makes a project; returns its number.
pub fn project(db: &UserDb, fields: &ProjectFields) -> Result<i64, StoreError> {
    check(db, None, fields)?;

    Ok(store::project(db, fields)?)
}
