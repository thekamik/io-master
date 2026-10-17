//! Changing a scheme of an open test project: written through to the
//! database, then made in memory.

use crate::db::test_projects::update as store;
use crate::db::user::database::UserDb;

use crate::model::test_projects::rules::SchemeError;
use crate::model::test_projects::{ProposalState, SchemeFields};
use crate::test_projects::error::StoreError;
use crate::test_projects::workspace::OpenProject;

use super::create::check;
use super::read::existing;

/// Changes the fields of scheme `id`.
pub fn scheme(db: &UserDb, open: &mut OpenProject, id: i64, fields: &SchemeFields) -> Result<(), StoreError> {
    // The normal form (model/): trimmed here, stored as it is.
    let fields = &fields.normalized();
    existing(open, id)?;
    check(db, open, Some(id), fields)?;

    store::scheme(db, id, fields)?;

    if let Some(scheme) = open.data_mut().schemes.iter_mut().find(|scheme| scheme.id == id) {
        scheme.fields = fields.clone();
    }

    Ok(())
}

/// Accepts a scheme proposed by the AI.
pub fn accept(db: &UserDb, open: &mut OpenProject, id: i64) -> Result<(), StoreError> {
    existing(open, id)?;

    if !store::accept_scheme(db, id)? {
        return Err(SchemeError::NotFound("scheme").into());
    }

    if let Some(scheme) = open.data_mut().schemes.iter_mut().find(|scheme| scheme.id == id) {
        scheme.fields.state = ProposalState::Accepted;
    }

    Ok(())
}
