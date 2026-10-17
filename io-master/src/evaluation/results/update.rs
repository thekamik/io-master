//! The results of an open evaluation made or changed by the evaluator:
//! a proposal accepted (unchanged or corrected), or a result written
//! alone. Every change is written through with its entry of the
//! journal, then made in memory.

use crate::db::evaluations::create::Step;
use crate::db::evaluations::update as store;
use crate::db::user::database::UserDb;
use crate::evaluation::error::EvaluationError;
use crate::evaluation::workspace::OpenEvaluation;
use crate::model::evaluations::{Actor, JournalAction, Proposal, RequirementResult, ResultField, ResultOrigin};

use super::read::{check_result, proposal, requirement, text_of};

/// The detail of the journal for a result that replaces `earlier`.
fn replacing(earlier: Option<&str>, text: &str) -> String {
    match earlier {
        Some(earlier) => format!("{text}\n\n--- replaces the earlier result ---\n{earlier}"),
        None => text.to_owned(),
    }
}

/// Puts `text` in memory as the result of `field` of `tile`, replacing
/// an earlier one; `None` removes it.
fn keep(open: &mut OpenEvaluation, tile: i64, field: ResultField, text: Option<&str>, origin: ResultOrigin) {
    let results = open.results_mut();
    results.retain(|result| !(result.tile == tile && result.field == field));

    if let Some(text) = text {
        results.push(RequirementResult { tile, field, text: text.to_owned(), origin });
    }
}

/// Makes `text` the result of the field of `proposal` and removes the
/// proposal, with the entry of the journal.
fn settle(
    db: &UserDb,
    open: &mut OpenEvaluation,
    proposal: &Proposal,
    text: &str,
    origin: ResultOrigin,
    action: JournalAction,
    detail: &str,
) -> Result<(), EvaluationError> {
    requirement(open, proposal.tile)?;

    let evaluation = open.data().evaluation.id;
    let step =
        Step { actor: &Actor::Evaluator, action, tile: Some(proposal.tile), field: Some(proposal.field), detail };

    store::settle_proposal(db, evaluation, proposal, text, origin, &step)?;

    keep(open, proposal.tile, proposal.field, Some(text), origin);
    open.proposals_mut().retain(|known| known.id != proposal.id);

    Ok(())
}

/// The evaluator accepts a proposal unchanged: its text becomes the
/// result of its field, replacing an earlier one.
pub fn accept(db: &UserDb, open: &mut OpenEvaluation, id: i64) -> Result<(), EvaluationError> {
    let proposal = proposal(open, id)?;
    let earlier = text_of(open, proposal.tile, proposal.field);
    let detail = replacing(earlier.as_deref(), &proposal.text);

    settle(db, open, &proposal, &proposal.text, ResultOrigin::Ai, JournalAction::Accepted, &detail)
}

/// The evaluator corrects a proposal and accepts it as `text`. A text
/// equal to the proposal is an acceptance unchanged.
pub fn correct(db: &UserDb, open: &mut OpenEvaluation, id: i64, text: &str) -> Result<(), EvaluationError> {
    let proposal = proposal(open, id)?;

    if text == proposal.text {
        return accept(db, open, id);
    }

    if text.trim().is_empty() {
        let reason = "the corrected text is empty; reject the proposal instead".to_owned();
        return Err(EvaluationError::Invalid("text", reason));
    }

    check_result(text)?;

    let earlier = text_of(open, proposal.tile, proposal.field);
    let corrected = format!("{text}\n\n--- corrected from the proposal ---\n{}", proposal.text);
    let detail = replacing(earlier.as_deref(), &corrected);

    settle(db, open, &proposal, text, ResultOrigin::AiCorrected, JournalAction::Corrected, &detail)
}

/// The evaluator writes the result of `field` alone, replacing an
/// earlier one; an empty text removes the result. Proposals waiting
/// for the field are not touched.
pub fn write(
    db: &UserDb,
    open: &mut OpenEvaluation,
    tile: i64,
    field: ResultField,
    text: &str,
) -> Result<(), EvaluationError> {
    requirement(open, tile)?;
    check_result(text)?;

    let earlier = text_of(open, tile, field);

    if earlier.as_deref() == Some(text) || (earlier.is_none() && text.trim().is_empty()) {
        return Ok(());
    }

    let removed = text.trim().is_empty();
    let detail = if removed {
        format!("--- removed the result ---\n{}", earlier.unwrap_or_default())
    } else {
        replacing(earlier.as_deref(), text)
    };
    let step = Step {
        actor: &Actor::Evaluator,
        action: JournalAction::Written,
        tile: Some(tile),
        field: Some(field),
        detail: &detail,
    };
    let text = if removed { None } else { Some(text) };

    store::result(db, open.data().evaluation.id, tile, text, &step)?;
    keep(open, tile, field, text, ResultOrigin::Evaluator);

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// cover the whole folder (create, read, update, delete).
#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
