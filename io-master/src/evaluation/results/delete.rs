//! The evaluator rejects a proposal of the AI; the result of its field
//! is not touched.

use crate::db::evaluations::delete as store;
use crate::db::evaluations::journal::create::JournalStep;
use crate::db::user::database::UserDb;
use crate::evaluation::error::EvaluationError;
use crate::evaluation::workspace::OpenEvaluation;
use crate::model::evaluations::{Actor, JournalAction};

use super::read::proposal;

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

    store::proposal(db, evaluation, id, &step)?;
    open.proposals_mut().retain(|known| known.id != id);

    Ok(())
}
