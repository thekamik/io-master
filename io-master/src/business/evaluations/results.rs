//! The results of the requirements of an open evaluation and the
//! proposals of the AI for them, as a resource (step 29 of
//! `docs/todo.md`, operations named in step 40). Every requirement has
//! four text results (`model/evaluations.rs`, `ResultField`).
//!
//! ```text
//! list()        GET     /requirements/{tile}/results
//! proposals()   GET     /requirements/{tile}/proposals
//! propose()     POST    /requirements/{tile}/proposals   the AI writes one: kept apart, marked AI
//! accept()      PUT     /proposals/{id}/accepted         taken unchanged     → result, origin ai
//! correct()     PUT     /proposals/{id}/corrected        corrected and taken → result, origin ai_corrected
//! write()       PUT     /requirements/{tile}/results/{field}  written alone → result, origin evaluator
//! reject()      DELETE  /proposals/{id}                  thrown away; the result is not touched
//! ```
//!
//! **A proposal never becomes a result on its own** (user, 2026-10-17):
//! only `accept` and `correct`, actions of the evaluator, turn it into
//! one. A new proposal does not touch an accepted result; it waits next
//! to it. Every step is written through to the database with its entry
//! of the journal, in one transaction (`db/evaluations/`), and only then
//! made in memory.

use crate::business::evaluations::error::EvaluationError;
use crate::business::evaluations::workspace::OpenEvaluation;
use crate::db::evaluations as db_evaluations;
use crate::db::evaluations::journal::create::JournalStep;
use crate::db::user::database::UserDb;
use crate::model::evaluations::{
    Actor, JournalAction, MAX_RESULT, MAX_SHORT, Proposal, RequirementResult, ResultField, ResultOrigin,
};
use crate::model::test_projects::TileKind;
use crate::model::text::{Entity, Field, check_text};
use crate::model::time::now;

/* ---------------------------------------------------------------
 * Read
 * ------------------------------------------------------------- */

/// Checks that `tile` is a requirement of the open evaluation.
pub(super) fn requirement(open: &OpenEvaluation, tile: i64) -> Result<(), EvaluationError> {
    let tile =
        open.data().tiles.iter().find(|known| known.id == tile).ok_or(EvaluationError::NotFound(Entity::Tile))?;

    if tile.fields.kind != TileKind::Requirement {
        return Err(EvaluationError::NotARequirement);
    }

    Ok(())
}

/// Checks the text of a result or a proposal.
pub(super) fn check_result(text: &str) -> Result<(), EvaluationError> {
    Ok(check_text(Field::Text, text, MAX_RESULT, true)?)
}

/// The accepted results of requirement `tile`, in the order of
/// [`ResultField::ALL`]; a field without a result is left out.
pub fn list(open: &OpenEvaluation, tile: i64) -> Vec<RequirementResult> {
    let mut results: Vec<RequirementResult> =
        open.results().iter().filter(|result| result.tile == tile).cloned().collect();

    results.sort_by_key(|result| ResultField::ALL.iter().position(|field| *field == result.field));

    results
}

/// The accepted text of a field, if any.
pub(super) fn text_of(open: &OpenEvaluation, tile: i64, field: ResultField) -> Option<String> {
    open.results().iter().find(|result| result.tile == tile && result.field == field).map(|result| result.text.clone())
}

/// The proposals of the AI waiting for requirement `tile`, the oldest
/// first.
pub fn proposals(open: &OpenEvaluation, tile: i64) -> Vec<Proposal> {
    open.proposals().iter().filter(|proposal| proposal.tile == tile).cloned().collect()
}

/// Proposal `id` of the open evaluation; the error that it has no such.
pub(super) fn proposal(open: &OpenEvaluation, id: i64) -> Result<Proposal, EvaluationError> {
    open.proposals()
        .iter()
        .find(|proposal| proposal.id == id)
        .cloned()
        .ok_or(EvaluationError::NotFound(Entity::Proposal))
}

/* ---------------------------------------------------------------
 * Create
 * ------------------------------------------------------------- */

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
        return Err(EvaluationError::Invalid(Field::Text, "the proposal is empty".to_owned()));
    }

    check_result(text)?;
    check_text(Field::Model, model, MAX_SHORT, false)?;

    let evaluation = open.data().evaluation.id;
    let actor = Actor::Model(model.to_owned());
    let step = JournalStep {
        actor: &actor,
        action: JournalAction::Proposed,
        tile: Some(tile),
        field: Some(field),
        detail: text,
    };

    let id = db_evaluations::create::proposal(db, evaluation, tile, field, text, model, &step)?;

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

/* ---------------------------------------------------------------
 * Update
 * ------------------------------------------------------------- */

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
    let step = JournalStep {
        actor: &Actor::Evaluator,
        action,
        tile: Some(proposal.tile),
        field: Some(proposal.field),
        detail,
    };

    db_evaluations::update::settle_proposal(db, evaluation, proposal, text, origin, &step)?;

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
        return Err(EvaluationError::Invalid(Field::Text, reason));
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
    let step = JournalStep {
        actor: &Actor::Evaluator,
        action: JournalAction::Written,
        tile: Some(tile),
        field: Some(field),
        detail: &detail,
    };
    let text = if removed { None } else { Some(text) };

    db_evaluations::update::result(db, open.data().evaluation.id, tile, field, text, &step)?;
    keep(open, tile, field, text, ResultOrigin::Evaluator);

    Ok(())
}

/* ---------------------------------------------------------------
 * Delete
 * ------------------------------------------------------------- */

/// Rejects proposal `id`.
pub fn reject(db: &UserDb, open: &mut OpenEvaluation, id: i64) -> Result<(), EvaluationError> {
    let proposal = proposal(open, id)?;
    let evaluation = open.data().evaluation.id;
    let step = JournalStep {
        actor: &Actor::Evaluator,
        action: JournalAction::Rejected,
        tile: Some(proposal.tile),
        field: Some(proposal.field),
        detail: &proposal.text,
    };

    db_evaluations::delete::proposal(db, evaluation, id, &step)?;
    open.proposals_mut().retain(|known| known.id != id);

    Ok(())
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "results_tests.rs"]
mod tests;
