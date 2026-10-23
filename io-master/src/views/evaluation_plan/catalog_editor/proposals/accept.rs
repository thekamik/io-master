//! A proposal of the AI accepted into its catalog.

use serde_json::Value;

use crate::db::evaluation_support as db_evaluation_support;
use crate::db::oscal as db_oscal;
use crate::db::oscal::error::OscalError;
use crate::db::user::database::{UserDb, in_transaction};
use crate::model::evaluation_support::{Proposal, ProposalKind};
use crate::model::oscal::catalog::edit::Place;
use crate::model::oscal::catalog::{Control, Group};

/// Accepts proposal `id` into its catalog: as it was proposed, or as
/// `corrected` (its JSON of OSCAL, changed by the evaluator). Returns the
/// id of the object.
///
/// One transaction (`in_transaction` takes the work as a closure: the
/// database keeps its connection to itself, so the transaction is opened
/// there): the object added, its quotes kept, the proposal removed; a
/// failure leaves all three as they were.
pub fn accept(db: &UserDb, id: i64, corrected: Option<Value>) -> Result<String, OscalError> {
    let Some(proposal) = db_evaluation_support::read::proposal(db, id)? else {
        return Err(OscalError::Invalid("the proposal does not exist".to_owned()));
    };

    let content = match corrected {
        Some(content) => content,
        None => proposal.content.clone(),
    };

    in_transaction(db, || {
        let object = add(db, &proposal, content)?;

        for quote in &proposal.quotes {
            db_evaluation_support::create::quote(db, &proposal.document_uuid, &object, quote)?;
        }

        db_evaluation_support::delete::proposal(db, id)?;

        Ok(object)
    })
}

/// Adds the object `content` of `proposal` to its catalog; returns its id.
fn add(db: &UserDb, proposal: &Proposal, content: Value) -> Result<String, OscalError> {
    let place = match &proposal.parent_id {
        Some(parent) => Place::Group(parent.clone()),
        None => Place::Top,
    };

    let mut catalog = db_oscal::read::catalog(db, &proposal.document_uuid)?;

    let object = match proposal.kind {
        ProposalKind::Group => {
            let group: Group = read(content)?;
            let object = group.id.clone().unwrap_or_default();
            catalog.add_group(&place, group)?;
            object
        }
        ProposalKind::Control => {
            let control: Control = read(content)?;
            let object = control.id.clone();
            catalog.add_control(&place, control)?;
            object
        }
    };

    db_oscal::update::catalog(db, catalog)?;

    Ok(object)
}

/// `content` read as an object of OSCAL.
fn read<T: serde::de::DeserializeOwned>(content: Value) -> Result<T, OscalError> {
    match serde_json::from_value(content) {
        Ok(object) => Ok(object),
        Err(error) => Err(OscalError::Invalid(format!("the object is not OSCAL: {error}"))),
    }
}
