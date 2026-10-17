//! Reading the results and the proposals of a requirement of an open
//! evaluation, from memory; and the checks the other files share.
//!
//! The results of the requirements: four text fields per requirement
//! (`model.rs`, `ResultField`), and the texts the AI proposes for them:
//!
//! ```text
//! create.rs   propose()   the AI writes a proposal: kept apart, marked AI
//! update.rs   accept()    the evaluator takes it unchanged     → result, origin ai
//!             correct()   the evaluator corrects and takes it  → result, origin ai_corrected
//!             write()     the evaluator writes the result alone → result, origin evaluator
//! delete.rs   reject()    the evaluator throws a proposal away
//! ```
//!
//! **A proposal never becomes a result on its own** (user,
//! 2026-10-17): only `accept` and `correct`, actions of the evaluator,
//! turn it into one. A new proposal does not touch an accepted result;
//! it waits next to it. Every step is written through to the database
//! with its entry of the journal, in one transaction
//! (`db/evaluations/`), and only then made in memory.

use crate::evaluation::model::{
    EvaluationError, MAX_RESULT, Proposal, RequirementResult, ResultField, check_text,
};
use crate::evaluation::workspace::OpenEvaluation;
use crate::test_projects::model::TileKind;

/// Checks that `tile` is a requirement of the open evaluation.
pub(super) fn requirement(open: &OpenEvaluation, tile: i64) -> Result<(), EvaluationError> {
    let tile = open
        .data()
        .tiles
        .iter()
        .find(|known| known.id == tile)
        .ok_or(EvaluationError::NotFound("tile"))?;

    if tile.fields.kind != TileKind::Requirement {
        return Err(EvaluationError::NotARequirement);
    }

    Ok(())
}

/// Checks the text of a result or a proposal.
pub(super) fn check_result(text: &str) -> Result<(), EvaluationError> {
    check_text("text", text, MAX_RESULT, true)
}

/// The accepted results of requirement `tile`, in the order of
/// [`ResultField::ALL`]; a field without a result is left out.
pub fn of(open: &OpenEvaluation, tile: i64) -> Vec<RequirementResult> {
    let mut results: Vec<RequirementResult> =
        open.results().iter().filter(|result| result.tile == tile).cloned().collect();

    results.sort_by_key(|result| ResultField::ALL.iter().position(|field| *field == result.field));

    results
}

/// The accepted text of a field, if any.
pub(super) fn text_of(open: &OpenEvaluation, tile: i64, field: ResultField) -> Option<String> {
    open.results()
        .iter()
        .find(|result| result.tile == tile && result.field == field)
        .map(|result| result.text.clone())
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
        .ok_or(EvaluationError::NotFound("proposal"))
}
