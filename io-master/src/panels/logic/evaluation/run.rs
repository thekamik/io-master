//! The assessment of an evaluation by the AI (user, 2026-10-30): every work
//! unit, in the order of the work, without the evaluator, through the night.
//!
//! ```text
//! units_to_assess ──► the work units in the order of the work (Kahn: first those without dependencies,
//!                     then those whose dependencies are assessed); a work unit of author User never,
//!                     one with a verdict only with "again"
//! preview         ──► how many questions go to which agents; every document and proof they need that is
//!                     not given: the assessment does not start while one is missing
//! for each:  prepared_assessment ──► the question with the documents whole, or their passages when
//!                                    they do not fit (ask::prepare tells it)
//!            assessed            ──► read_assessment, apply_assessment
//! ```

use crate::ai::ask::{Prepared, prepare};
use crate::ai::error::AiError;
use crate::db::ai::agent::read_agent;
use crate::db::database::Database;
use crate::db::evaluation::lab_proof::read_lab_proof;
use crate::db::evaluation::product_document::read_product_document;
use crate::db::evaluation::work_unit::read_work_units;
use crate::model::evaluation::work_unit::{Author, WorkUnit};
use crate::panels::logic::evaluation::answer::{apply_assessment, read_assessment};
use crate::panels::logic::evaluation::error::EvaluationError;
use crate::panels::logic::evaluation::evaluations::evaluation_in_progress;
use crate::panels::logic::evaluation::question::{assessing_agent, assessment_question};
use crate::panels::logic::evaluation_plan::order::work_order;

/// What an assessment would ask, counted when the evaluator asks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// The work units to assess, in the order of the work.
    pub units: Vec<i64>,
    /// Each agent that would be asked, with how many questions.
    pub agents: Vec<(String, usize)>,
    /// Each document or proof needed and not given, with how many of the
    /// work units need it.
    pub missing: Vec<(String, usize)>,
}

/// The work units of the evaluation `evaluation` in the order of the work.
///
/// Called by: [`units_to_assess`]; `panels/views/evaluation/work_units.rs`
/// (the list of the work units).
/// Calls: `read_work_units`, `work_order`.
pub fn units_in_order(db: &Database, evaluation: i64) -> Result<Vec<WorkUnit>, EvaluationError> {
    let units = read_work_units(db, evaluation)?;
    let order = work_order(&units);

    ensure!(order.crossed.is_empty(), EvaluationError::Refused("the evaluation has a cross reference".to_owned()));

    let mut ordered = Vec::with_capacity(units.len());

    for position in order.order {
        if let Some(unit) = units.get(position) {
            ordered.push(unit.clone());
        }
    }

    Ok(ordered)
}

/// The work units of the evaluation `evaluation` to assess, in the order of
/// the work: never one of author User (the evaluator decides, P1); one with
/// a verdict only with `again`.
///
/// Called by: [`preview`].
/// Calls: `evaluation_in_progress`, [`units_in_order`].
fn units_to_assess(db: &Database, evaluation: i64, again: bool) -> Result<Vec<WorkUnit>, EvaluationError> {
    evaluation_in_progress(db, evaluation)?;

    let mut chosen = Vec::new();

    for unit in units_in_order(db, evaluation)? {
        if unit.author != Some(Author::User) && (again || unit.verdict.is_none()) {
            chosen.push(unit);
        }
    }

    Ok(chosen)
}

/// Counts `name` once more in `counted`.
///
/// Called by: [`preview`].
fn count(counted: &mut Vec<(String, usize)>, name: String) {
    for (known, number) in counted.iter_mut() {
        if *known == name {
            *number += 1;
            return;
        }
    }

    counted.push((name, 1));
}

/// The inputs the work unit `unit` needs and that are not given, by their
/// names.
///
/// Called by: [`preview`].
/// Calls: `read_product_document`, `read_lab_proof`.
fn missing_inputs(db: &Database, unit: &WorkUnit) -> Result<Vec<String>, EvaluationError> {
    let mut missing = Vec::new();

    for id in &unit.input_product_documents {
        let document = read_product_document(db, *id)?;

        if document.content.is_none() {
            missing.push(format!("the document {}", document.name));
        }
    }

    for id in unit.input_functional.as_deref().unwrap_or_default() {
        let proof = read_lab_proof(db, *id)?;

        if proof.proof.is_none() {
            missing.push(format!("the proof {}", proof.name));
        }
    }

    Ok(missing)
}

/// What the assessment of the evaluation `evaluation` would ask, counted
/// now: its work units, the agents asked, the inputs missing.
///
/// Called by: `panels/views/evaluation/run.rs` (the preview, the start).
/// Calls: [`units_to_assess`], `evaluation_in_progress`, `assessing_agent`,
/// `read_agent`, [`missing_inputs`], [`count`].
pub fn preview(db: &Database, evaluation: i64, again: bool) -> Result<Preview, EvaluationError> {
    let units = units_to_assess(db, evaluation, again)?;
    let stored = evaluation_in_progress(db, evaluation)?;
    let mut found = Preview { units: Vec::with_capacity(units.len()), agents: Vec::new(), missing: Vec::new() };

    for unit in &units {
        found.units.push(unit.id);
        count(&mut found.agents, read_agent(db, assessing_agent(&stored, unit)?)?.name);

        for name in missing_inputs(db, unit)? {
            count(&mut found.missing, name);
        }
    }

    Ok(found)
}

/// The question of the work unit `unit`, prepared: with its documents whole,
/// or with their passages when they do not fit into the context of the model.
///
/// Called by: `panels/views/evaluation/run.rs` (each work unit).
/// Calls: `assessment_question`, `ask::prepare`.
pub fn prepared_assessment(db: &Database, evaluation: i64, unit: i64) -> Result<Prepared, EvaluationError> {
    match prepare(db, &assessment_question(db, evaluation, unit, true)?) {
        Ok(prepared) => Ok(prepared),
        Err(AiError::TooLong { .. }) => Ok(prepare(db, &assessment_question(db, evaluation, unit, false)?)?),
        Err(error) => Err(EvaluationError::Ai(error)),
    }
}

/// Reads the answer `answer` and writes it into the work unit `unit`; gives
/// the notes of what was not taken.
///
/// Called by: `panels/views/evaluation/run.rs` (each answer, after
/// `ask::record`).
/// Calls: `read_assessment`, `apply_assessment`.
pub fn assessed(db: &Database, evaluation: i64, unit: i64, answer: &str) -> Result<Vec<String>, EvaluationError> {
    apply_assessment(db, evaluation, unit, &read_assessment(answer)?)
}

#[cfg(test)]
mod tests {
    //! Tests of the run: the work units in the order of the work, those of
    //! the user passed over; the inputs missing; a whole answer written.

    use super::*;

    use crate::db::evaluation::evaluation_plan::{create_evaluation, read_evaluation_plan};
    use crate::db::evaluation::work_unit::read_work_unit;
    use crate::model::evaluation::work_unit::Verdict;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::{evaluation, write};

    #[test]
    fn the_work_units_come_in_the_order_of_the_work_and_those_of_the_user_are_passed_over() {
        let (_directory, db) = database();
        let (id, first, second) = evaluation(&db);
        let found = preview(&db, id, false).unwrap();

        assert_eq!(found.units, vec![first, second]);
        assert_eq!(found.agents, vec![("CC evaluator".to_owned(), 2)]);
        assert!(found.missing.is_empty());

        let answer = "{\"verdict\": \"pass\", \"analysis\": \"ok\", \"quotes\": []}";

        assert!(assessed(&db, id, first, answer).unwrap().is_empty());
        assert_eq!(preview(&db, id, false).unwrap().units, vec![second]);
        assert_eq!(preview(&db, id, true).unwrap().units, vec![first, second]);

        let corrected = WorkUnit {
            verdict: Some(Verdict::Fail),
            author: Some(Author::User),
            ..read_work_unit(&db, first).unwrap()
        };

        write(&db, &corrected);
        assert_eq!(preview(&db, id, true).unwrap().units, vec![second]);
    }

    #[test]
    fn the_inputs_missing_are_counted_and_a_question_is_prepared() {
        let (_directory, db) = database();
        let (id, first, _) = evaluation(&db);
        let source = read_evaluation_plan(&db, id).unwrap().source_plan.unwrap();
        let agent = read_evaluation_plan(&db, id).unwrap().ai_agent;
        let bare = create_evaluation(&db, source, "R8", "ACME", "K. K.", 1, agent).unwrap();

        assert_eq!(preview(&db, bare, false).unwrap().missing, vec![("the document ST".to_owned(), 2)]);

        // The integration of the tests is local, needing no key.
        assert!(prepared_assessment(&db, id, first).is_ok());
    }
}
