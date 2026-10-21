//! The proposals of the AI for an OSCAL document (evaluation support,
//! `model/evaluation_support.rs`), as a resource: they wait for the
//! evaluator and enter the document only when the evaluator accepts
//! them (P1 of `docs/architecture.md`: the evaluator decides).
//!
//! ```text
//! list()     GET     /oscal/{uuid}/proposals
//! accept()   POST    /proposals/{id}/accepted   as proposed, or as corrected by the evaluator
//! reject()   DELETE  /proposals/{id}
//! ```

use serde_json::Value;

use crate::db::error::DbError;
use crate::db::evaluation_support as db_evaluation_support;
use crate::db::user::database::{UserDb, in_transaction};
use crate::model::evaluation_support::{Proposal, ProposalKind};
use crate::model::oscal::catalog::{Control, Group};

use super::catalogs::{self, Place};
use crate::db::oscal::error::OscalError;

/// The proposals waiting for document `uuid`, the oldest first.
pub fn list(db: &UserDb, uuid: &str) -> Result<Vec<Proposal>, DbError> {
    db_evaluation_support::read::proposals(db, uuid)
}

/// Accepts proposal `id` into its document: as it was proposed, or as
/// `corrected` (its JSON of OSCAL, changed by the evaluator). In one
/// transaction: the object added (checked against the schema of NIST),
/// its quotes kept as evaluation support, the proposal removed. Returns
/// the id of the object.
pub fn accept(db: &UserDb, id: i64, corrected: Option<Value>) -> Result<String, OscalError> {
    let proposal = db_evaluation_support::read::proposal(db, id)?
        .ok_or(OscalError::Invalid("the proposal does not exist".to_owned()))?;
    let content = corrected.unwrap_or(proposal.content.clone());
    let place = match &proposal.parent_id {
        Some(parent) => Place::Group(parent.clone()),
        None => Place::Top,
    };
    let unreadable = |error: serde_json::Error| OscalError::Invalid(format!("the object is not OSCAL: {error}"));

    in_transaction(db, || {
        let object = match proposal.kind {
            ProposalKind::Group => {
                let group: Group = serde_json::from_value(content).map_err(unreadable)?;
                let object = group.id.clone().unwrap_or_default();
                catalogs::add_group(db, &proposal.document_uuid, &place, group)?;
                object
            }
            ProposalKind::Control => {
                let control: Control = serde_json::from_value(content).map_err(unreadable)?;
                let object = control.id.clone();
                catalogs::add_control(db, &proposal.document_uuid, &place, control)?;
                object
            }
        };

        for quote in &proposal.quotes {
            db_evaluation_support::create::quote(db, &proposal.document_uuid, &object, quote)?;
        }

        db_evaluation_support::delete::proposal(db, id)?;

        Ok(object)
    })
}

/// Rejects proposal `id`: it is removed, the document is not touched.
pub fn reject(db: &UserDb, id: i64) -> Result<bool, DbError> {
    db_evaluation_support::delete::proposal(db, id)
}
