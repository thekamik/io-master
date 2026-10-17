//! The results of the requirements of an evaluation: four text
//! fields per requirement (`model.rs`, [`ResultField`]), and the texts
//! the AI proposes for them.
//!
//! ```text
//! propose()   the AI writes a proposal: kept apart, marked AI
//!    │
//!    ├─ accept()    the evaluator takes it unchanged     → result, origin ai
//!    ├─ correct()   the evaluator corrects and takes it  → result, origin ai_corrected
//!    └─ reject()    the evaluator throws it away
//! write()     the evaluator writes the result alone      → result, origin evaluator
//! ```
//!
//! **A proposal never becomes a result on its own** (user,
//! 2026-10-17): only `accept` and `correct`, actions of the evaluator,
//! turn it into one. A new proposal does not touch an accepted result;
//! it waits next to it. Every step is written into the journal in the
//! same transaction, with the texts it concerns (the SQL is in
//! `db/evaluations/`).

use crate::db::error::DbError;
use crate::db::evaluations::create::{self as store, Step};
use crate::db::evaluations::{delete as remove, read, update as change};
use crate::db::user::database::UserDb;
use crate::evaluation::scheme::TileKind;

use super::model::{
    Actor, EvaluationError, JournalAction, MAX_RESULT, MAX_SHORT, Proposal, RequirementResult, ResultField,
    ResultOrigin, check_text,
};
use super::projects;

/// The evaluation of `tile`, when it is a requirement of one.
fn requirement(db: &UserDb, tile: i64) -> Result<i64, EvaluationError> {
    let (evaluation, tile) = projects::tile(db, tile)?.ok_or(EvaluationError::NotFound("tile"))?;

    if tile.fields.kind != TileKind::Requirement {
        return Err(EvaluationError::NotARequirement);
    }

    Ok(evaluation)
}

/// Checks the text of a result or a proposal.
fn check_result(text: &str) -> Result<(), EvaluationError> {
    check_text("text", text, MAX_RESULT, true)
}

/// The accepted results of a requirement, in the order of
/// [`ResultField::ALL`]; a field without a result is left out.
pub fn of(db: &UserDb, tile: i64) -> Result<Vec<RequirementResult>, DbError> {
    let mut results: Vec<RequirementResult> = read::results(db, tile)?
        .into_iter()
        .filter_map(|(field, text, origin)| {
            Some(RequirementResult {
                tile,
                field: ResultField::from_name(&field)?,
                text,
                // Only valid names are written; a row changed by hand
                // is shown as the evaluator's, the safe side.
                origin: ResultOrigin::from_name(&origin).unwrap_or(ResultOrigin::Evaluator),
            })
        })
        .collect();

    results.sort_by_key(|result| ResultField::ALL.iter().position(|field| *field == result.field));

    Ok(results)
}

/// The proposals of the AI waiting for the evaluator, for a
/// requirement, the oldest first.
pub fn proposals(db: &UserDb, tile: i64) -> Result<Vec<Proposal>, DbError> {
    read::proposals(db, tile)
}

/// One proposal; the error that it does not exist otherwise.
fn proposal(db: &UserDb, id: i64) -> Result<Proposal, EvaluationError> {
    let tile = read::proposal_tile(db, id)?.ok_or(EvaluationError::NotFound("proposal"))?;

    proposals(db, tile)?
        .into_iter()
        .find(|proposal| proposal.id == id)
        .ok_or(EvaluationError::NotFound("proposal"))
}

/// Stores a text the AI proposes for `field` of requirement `tile`;
/// `model` names the model that wrote it. Returns the number of the
/// proposal.
pub fn propose(db: &UserDb, tile: i64, field: ResultField, text: &str, model: &str) -> Result<i64, EvaluationError> {
    let evaluation = requirement(db, tile)?;

    if text.trim().is_empty() {
        return Err(EvaluationError::Invalid("text", "the proposal is empty".to_owned()));
    }

    check_result(text)?;
    check_text("model", model, MAX_SHORT, false)?;

    let actor = Actor::Model(model.to_owned());
    let step = Step {
        actor: &actor,
        action: JournalAction::Proposed,
        tile: Some(tile),
        field: Some(field),
        detail: text,
    };

    Ok(store::proposal(db, evaluation, tile, field, text, model, &step)?)
}

/// The detail of the journal for a result that replaces `earlier`.
fn replacing(earlier: Option<&str>, text: &str) -> String {
    match earlier {
        Some(earlier) => format!("{text}\n\n--- replaces the earlier result ---\n{earlier}"),
        None => text.to_owned(),
    }
}

/// Makes `text` the result of the field of `proposal` and removes the
/// proposal, with the entry of the journal.
fn settle(
    db: &UserDb,
    proposal: &Proposal,
    text: &str,
    origin: ResultOrigin,
    action: JournalAction,
    detail: &str,
) -> Result<(), EvaluationError> {
    let evaluation = requirement(db, proposal.tile)?;
    let step = Step {
        actor: &Actor::Evaluator,
        action,
        tile: Some(proposal.tile),
        field: Some(proposal.field),
        detail,
    };

    Ok(change::settle_proposal(db, evaluation, proposal, text, origin, &step)?)
}

/// The evaluator accepts a proposal unchanged: its text becomes the
/// result of its field, replacing an earlier one.
pub fn accept(db: &UserDb, id: i64) -> Result<(), EvaluationError> {
    let proposal = proposal(db, id)?;
    let earlier = read::result_text(db, proposal.tile, proposal.field)?;
    let detail = replacing(earlier.as_deref(), &proposal.text);

    settle(db, &proposal, &proposal.text, ResultOrigin::Ai, JournalAction::Accepted, &detail)
}

/// The evaluator corrects a proposal and accepts it as `text`. A text
/// equal to the proposal is an acceptance unchanged.
pub fn correct(db: &UserDb, id: i64, text: &str) -> Result<(), EvaluationError> {
    let proposal = proposal(db, id)?;

    if text == proposal.text {
        return accept(db, id);
    }

    if text.trim().is_empty() {
        let reason = "the corrected text is empty; reject the proposal instead".to_owned();
        return Err(EvaluationError::Invalid("text", reason));
    }

    check_result(text)?;

    let earlier = read::result_text(db, proposal.tile, proposal.field)?;
    let corrected = format!("{text}\n\n--- corrected from the proposal ---\n{}", proposal.text);
    let detail = replacing(earlier.as_deref(), &corrected);

    settle(db, &proposal, text, ResultOrigin::AiCorrected, JournalAction::Corrected, &detail)
}

/// The evaluator rejects a proposal; the result of its field is not
/// touched.
pub fn reject(db: &UserDb, id: i64) -> Result<(), EvaluationError> {
    let proposal = proposal(db, id)?;
    let evaluation = requirement(db, proposal.tile)?;
    let step = Step {
        actor: &Actor::Evaluator,
        action: JournalAction::Rejected,
        tile: Some(proposal.tile),
        field: Some(proposal.field),
        detail: &proposal.text,
    };

    Ok(remove::proposal(db, evaluation, id, &step)?)
}

/// The evaluator writes the result of `field` alone, replacing an
/// earlier one; an empty text removes the result. Proposals waiting
/// for the field are not touched.
pub fn write(db: &UserDb, tile: i64, field: ResultField, text: &str) -> Result<(), EvaluationError> {
    let evaluation = requirement(db, tile)?;

    check_result(text)?;

    let earlier = read::result_text(db, tile, field)?;

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

    Ok(change::result(db, evaluation, tile, if removed { None } else { Some(text) }, &step)?)
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "results_tests.rs"]
mod tests;
