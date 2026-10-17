//! The evaluation projects open for work: the objects in memory (user,
//! 2026-10-17), the same pattern as `test_projects/workspace.rs`.
//!
//! ```text
//! db/evaluations/  ──open()──►  OpenEvaluation in memory
//!                                data        the frozen structure (never changed)
//!                                results     the accepted results of the requirements
//!                                proposals   the texts of the AI waiting for the evaluator
//!                  ◄──every change, at once, with its entry of the journal── results/
//! ```
//!
//! **Where the information is.** While an evaluation is open, its
//! results and proposals are in memory and the same as in the
//! database: every change is written through at once, together with
//! its entry of the journal (`db/evaluations/`), and only then made in
//! memory. The journal itself is not kept in memory: it is read from
//! the database when it is shown (`journal/read.rs`).
//!
//! An evaluation has no "Undo": every step stays in its journal, and
//! a result is changed by a new step (written again, or a new proposal
//! accepted).

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::db::evaluations::read as stored;
use crate::db::user::database::UserDb;

use super::model::{EvaluationData, EvaluationError, Proposal, RequirementResult, ResultField, ResultOrigin};
use super::projects::read;

/// An evaluation open for work.
#[derive(Debug)]
pub struct OpenEvaluation {
    data: EvaluationData,
    results: Vec<RequirementResult>,
    proposals: Vec<Proposal>,
}

impl OpenEvaluation {
    /// Reads evaluation `id` from the database; `None` when there is no
    /// such.
    pub fn read(db: &UserDb, id: i64) -> Result<Option<OpenEvaluation>, EvaluationError> {
        let Some(data) = read::data(db, id)? else {
            return Ok(None);
        };

        let results = stored::results_of(db, id)?
            .into_iter()
            .filter_map(|(tile, field, text, origin)| {
                Some(RequirementResult {
                    tile,
                    field: ResultField::from_name(&field)?,
                    text,
                    // Only valid names are written; a row changed by
                    // hand is shown as the evaluator's, the safe side.
                    origin: ResultOrigin::from_name(&origin).unwrap_or(ResultOrigin::Evaluator),
                })
            })
            .collect();

        Ok(Some(OpenEvaluation {
            data,
            results,
            proposals: stored::proposals_of(db, id)?,
        }))
    }

    /// The frozen structure of the evaluation.
    pub fn data(&self) -> &EvaluationData {
        &self.data
    }

    /// The accepted results of every requirement.
    pub fn results(&self) -> &[RequirementResult] {
        &self.results
    }

    /// The proposals of the AI waiting for the evaluator, the oldest
    /// first.
    pub fn proposals(&self) -> &[Proposal] {
        &self.proposals
    }

    /// The results, to be changed by `results/` after the database has
    /// stored the change.
    pub(in crate::evaluation) fn results_mut(&mut self) -> &mut Vec<RequirementResult> {
        &mut self.results
    }

    /// The proposals, the same way.
    pub(in crate::evaluation) fn proposals_mut(&mut self) -> &mut Vec<Proposal> {
        &mut self.proposals
    }
}

/// The evaluations open for work of the logged-in evaluator.
#[derive(Debug, Default)]
pub struct Workspace {
    open: HashMap<i64, OpenEvaluation>,
}

impl Workspace {
    /// The open evaluation `id`; read from the database when it is not
    /// open yet.
    pub fn open(&mut self, db: &UserDb, id: i64) -> Result<&mut OpenEvaluation, EvaluationError> {
        match self.open.entry(id) {
            Entry::Occupied(open) => Ok(open.into_mut()),
            Entry::Vacant(place) => {
                let evaluation = OpenEvaluation::read(db, id)?.ok_or(EvaluationError::NotFound("evaluation"))?;
                Ok(place.insert(evaluation))
            }
        }
    }

    /// Closes evaluation `id` (it was deleted).
    pub fn close(&mut self, id: i64) {
        self.open.remove(&id);
    }

    /// Closes every evaluation: the evaluator logged out.
    pub fn clear(&mut self) {
        self.open.clear();
    }
}
