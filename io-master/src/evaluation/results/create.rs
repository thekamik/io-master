//! A text of the AI proposed for a field of a requirement of an open
//! evaluation: kept apart, marked AI, never a result until the
//! evaluator accepts it (`read.rs`).

use crate::db::evaluations::create as store;
use crate::db::evaluations::journal::create::JournalStep;
use crate::db::user::database::UserDb;
use crate::evaluation::error::EvaluationError;
use crate::evaluation::workspace::OpenEvaluation;
use crate::model::evaluations::{Actor, JournalAction, MAX_SHORT, Proposal, ResultField};
use crate::model::text::check_text;
use crate::model::time::now;

use super::read::{check_result, requirement};

/// Stores a text the AI proposes for `field` of requirement `tile`;
/// `model` names the model that wrote it. Returns the number of the
/// proposal.
pub fn propose(
    db: &UserDb,
    open: &mut OpenEvaluation,
    tile: i64,
    field: ResultField,
    text: &str,
    model: &str,
) -> Result<i64, EvaluationError> {
    requirement(open, tile)?;

    if text.trim().is_empty() {
        return Err(EvaluationError::Invalid("text", "the proposal is empty".to_owned()));
    }

    check_result(text)?;
    check_text("model", model, MAX_SHORT, false)?;

    let evaluation = open.data().evaluation.id;
    let actor = Actor::Model(model.to_owned());
    let step = JournalStep {
        actor: &actor,
        action: JournalAction::Proposed,
        tile: Some(tile),
        field: Some(field),
        detail: text,
    };

    let id = store::proposal(db, evaluation, tile, field, text, model, &step)?;

    open.proposals_mut().push(Proposal {
        id,
        tile,
        field,
        text: text.to_owned(),
        model: model.to_owned(),
        created: now(),
    });

    Ok(id)
}
