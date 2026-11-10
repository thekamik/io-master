//! The AI on a plan (`EvaluationPlan`): the assessment of an evaluation
//! (user, 2026-10-30), every work unit in the order of the work, without
//! the evaluator, through the night; "Fill with AI" of a plan in planning
//! (user, 2026-10-29), every work unit in the order of the plan.
//!
//! ```text
//! units_to_assess ──► the work units in the order of the work (Kahn: first those without dependencies,
//!                     then those whose dependencies are assessed); a work unit of author User never,
//!                     one with a verdict only with "again"
//! preview         ──► how many questions go to which agents; every document and proof they need that is
//!                     not given: the assessment does not start while one is missing
//! units_ready     ──► the work units to assess, refused while an input is missing
//! assess_evaluation ──► for each: work_unit_ai/assessment.rs: assess_work_unit ──► record_step (the progress)
//!
//! fill_units      ──► the work units of a plan in planning with an agent of the planner
//! fill_preview    ──► how many questions go to which agent and integration
//! fill_plan       ──► for each: work_unit_ai/filling.rs: fill_work_unit ──► record_step (the progress)
//! ```
//!
//! The loops take the lock of the database only for the short steps of each
//! work unit, never while the AI answers (`objects/session/progress/progress_ai/lock.rs`), so that the window
//! stays usable through the night.

use std::sync::Mutex;

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::integration::integration_store::read_integration;
use crate::objects::ai_error::AiError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::evaluation::evaluation_in_progress;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::order::work_order;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::plan_to_fill;
use crate::objects::evaluation::lab_proof::lab_proof_store::read_lab_proof;
use crate::objects::evaluation::product_document::product_document_store::read_product_document;
use crate::objects::evaluation::work_unit::work_unit_ai::assessment::assess_work_unit;
use crate::objects::evaluation::work_unit::work_unit_ai::assessment_question::assessing_agent;
use crate::objects::evaluation::work_unit::work_unit_ai::filling::{FillTask, fill_work_unit};
use crate::objects::evaluation::work_unit::work_unit_model::{Author, WorkUnit};
use crate::objects::evaluation::work_unit::work_unit_store::read_work_units;
use crate::objects::session::progress::progress_ai::lock::lock_progress;
use crate::objects::session::progress::progress_ai::step::record_step;
use crate::objects::session::progress::progress_model::Progress;

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
/// Input: `evaluation`, the id of a stored evaluation (a plan).
/// Output: its work units, each after its dependencies (`[ASE_INT.1-1,
/// ASE_INT.1-2, ...]`); `AiError::Refused` when it has a cross reference,
/// an error of the database.
///
/// Called by: [`units_to_assess`]; `panels/views/evaluation/work_units.rs`:
/// `evaluation_unit_list`.
/// Calls: `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/order.rs`: `work_order`.
pub fn units_in_order(db: &UserDatabase, evaluation: i64) -> Result<Vec<WorkUnit>, AiError> {
    let units = read_work_units(db, evaluation)?;
    let order = work_order(&units)?;

    ensure!(order.crossed.is_empty(), AiError::Refused("the evaluation has a cross reference".to_owned()));

    let mut ordered = Vec::with_capacity(units.len());

    for position in order.order {
        let unit = units.get(position).ok_or_else(|| {
            AiError::Refused(format!("the order of the work: the position {position} is outside the evaluation"))
        })?;

        ordered.push(unit.clone());
    }

    Ok(ordered)
}

/// The work units of the evaluation `evaluation` to assess, in the order of
/// the work: never one of author User (the evaluator decides, P1); one with
/// a verdict only with `again`.
///
/// Input: `evaluation`, the id of an evaluation in progress; `again`, whether
/// the work units with a verdict are assessed again.
/// Output: the work units chosen; `AiError::Refused` for an evaluation not in
/// progress or with a cross reference, an error of the database.
///
/// Called by: [`preview`].
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`: `evaluation_in_progress`;
/// [`units_in_order`].
fn units_to_assess(db: &UserDatabase, evaluation: i64, again: bool) -> Result<Vec<WorkUnit>, AiError> {
    evaluation_in_progress(db, evaluation)?;

    let mut chosen = Vec::new();

    for unit in units_in_order(db, evaluation)? {
        if unit.author != Some(Author::User) && (again || unit.verdict.is_none()) {
            chosen.push(unit);
        }
    }

    Ok(chosen)
}

/// Counts `name` once more in `counted`: `[("Evaluator", 2)]` becomes
/// `[("Evaluator", 3)]`, a new name is added with 1.
///
/// Input: `counted`, the names counted so far, at most one for each work
/// unit of an evaluation; `name`, the name to count.
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
/// names; a work unit that names no document and no proof needs one.
///
/// Input: `unit`, a work unit of an evaluation.
/// Output: the names (`["the document ST v1.2", "the proof Test 3"]`,
/// `["a document or a proof of ASE_INT.1-1"]`); an error of the database for
/// a document or a proof missing.
///
/// Called by: [`preview`].
/// Calls: `objects/evaluation/product_document/product_document_store.rs`: `read_product_document`;
/// `objects/evaluation/lab_proof/lab_proof_store.rs`: `read_lab_proof`.
fn missing_inputs(db: &UserDatabase, unit: &WorkUnit) -> Result<Vec<String>, AiError> {
    let mut missing = Vec::new();

    // Without evidence the work unit would be judged on nothing; the plan is
    // corrected before the assessment starts (user, 2026-11-02).
    if unit.input_product_documents.is_empty() && unit.input_functional.as_deref().unwrap_or_default().is_empty() {
        missing.push(format!("a document or a proof of {}", unit.label));
    }

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
/// Input: as [`units_to_assess`].
/// Output: the preview (`Preview { units: [7, 8], agents: [("Evaluator", 2)],
/// missing: [] }`); the errors of [`units_to_assess`], an error of the
/// database for an agent missing.
///
/// Called by: [`units_ready`]; `panels/views/evaluation/run.rs`:
/// `evaluation_assess_preview`.
/// Calls: [`units_to_assess`]; `objects/evaluation/evaluation_plan/evaluation_plan_rules/evaluation.rs`:
/// `evaluation_in_progress`; `objects/evaluation/work_unit/work_unit_ai/assessment_question.rs`: `assessing_agent`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`; [`missing_inputs`], [`count`].
pub fn preview(db: &UserDatabase, evaluation: i64, again: bool) -> Result<Preview, AiError> {
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

/// The work units of the plan `plan` to fill, in its order.
///
/// Input: `plan`, the id of a stored plan.
/// Output: the ids of its work units (`[3, 4, 5]`); `AiError::Refused`
/// unless the plan is in planning, names an agent of the planner and has
/// work units.
///
/// Called by: [`fill_preview`]; `panels/views/evaluation_plan/fill.rs`:
/// `fill` (the start of a filling).
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`.
pub fn fill_units(db: &UserDatabase, plan: i64) -> Result<Vec<i64>, AiError> {
    let (stored, _) = plan_to_fill(db, plan)?;

    ensure!(!stored.work_units.is_empty(), AiError::Refused("the plan has no work unit to fill".to_owned()));

    Ok(stored.work_units)
}

/// What a filling of the plan `plan` would ask, counted now: as many
/// questions as the plan has work units, to the integration of its agent of
/// the planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillPreview {
    pub questions: usize,
    pub agent: String,
    /// The name of the integration and its model.
    pub integration: String,
}

/// What a filling of the plan `plan` would ask, counted now, before the
/// planner starts it.
///
/// Input: `plan`, the id of a stored plan.
/// Output: the preview (`FillPreview { questions: 12, agent: "Planner",
/// integration: "Ollama · llama3.1" }`); refused as [`fill_units`] refuses.
///
/// Called by: `panels/views/evaluation_plan/fill.rs`: `plan_fill_preview`.
/// Calls: [`fill_units`]; `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_to_fill`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`; `objects/ai/integration/integration_store.rs`:
/// `read_integration`.
pub fn fill_preview(db: &UserDatabase, plan: i64) -> Result<FillPreview, AiError> {
    let questions = fill_units(db, plan)?.len();
    let (_, agent_id) = plan_to_fill(db, plan)?;
    let agent = read_agent(db, agent_id)?;
    let integration = read_integration(db, agent.integration)?;

    Ok(FillPreview {
        questions,
        agent: agent.name,
        integration: format!("{} · {}", integration.name, integration.model),
    })
}

/// The work units of the evaluation `evaluation` to assess, refused while a
/// document or a proof they need is not given.
///
/// Input: as [`units_to_assess`].
/// Output: their ids in the order of the work (`[7, 8]`); `AiError::Refused`
/// naming each input missing, the errors of [`preview`].
///
/// Called by: `panels/views/evaluation/run.rs`: `evaluation_assess` (the
/// start of an assessment).
/// Calls: [`preview`].
pub fn units_ready(db: &UserDatabase, evaluation: i64, again: bool) -> Result<Vec<i64>, AiError> {
    let found = preview(db, evaluation, again)?;
    let mut missing = Vec::with_capacity(found.missing.len());

    for (name, count) in &found.missing {
        missing.push(format!("{name} ({count} work units)"));
    }

    ensure!(
        missing.is_empty(),
        AiError::Refused(format!("give first what the work units need: {}", missing.join(", ")))
    );

    Ok(found.units)
}

/// Assesses the work units `units` of the evaluation `evaluation`, one after
/// the other in the order given (the order of the work, [`units_ready`]),
/// writing each step into `progress`; when the user asks it to stop, the
/// answer awaited ends (not taken) and no further work unit is begun.
///
/// Input:
/// - `database`: the lock of the open database of the session, taken only
///   for the short steps of each work unit, never while the AI answers.
/// - `progress`: the lock of the progress of the session; the work must be
///   begun in it (`Session::begin_work`).
/// - `evaluation`: an evaluation in progress; `units`: its work units.
///
/// Output: nothing; every work unit gone through is written into the
/// progress, assessed or not. An error ends the assessment only when the
/// session or the database fails; a work unit the AI could not assess is
/// written as a problem and passed over.
///
/// Called by: `panels/views/evaluation/run.rs`: `evaluation_assess`.
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_progress`;
/// `objects/evaluation/work_unit/work_unit_ai/assessment.rs`: `assess_work_unit`;
/// `objects/session/progress/progress_ai/step.rs`: `record_step`.
pub async fn assess_evaluation(
    database: &Mutex<Option<UserDatabase>>,
    progress: &Mutex<Progress>,
    evaluation: i64,
    units: &[i64],
) -> Result<(), AiError> {
    ensure!(lock_progress(progress)?.running, AiError::Refused("the assessment was not begun".to_owned()));

    for unit in units {
        if lock_progress(progress)?.stop {
            break;
        }

        let (label, verdict, notes) = assess_work_unit(database, progress, evaluation, *unit).await?;

        record_step(&mut *lock_progress(progress)?, label, verdict.is_some(), verdict, notes);
    }

    Ok(())
}

/// Fills the work units `units` of the plan `plan` with the task `task`, one
/// after the other in the order of the plan ([`fill_units`]), writing each
/// step into `progress`; when the user asks it to stop, the answer awaited
/// ends (not taken) and no further work unit is begun.
///
/// Input: as [`assess_evaluation`]; `plan` a plan in planning with an agent
/// of the planner, `units` its work units.
///
/// Output: nothing; every work unit gone through is written into the
/// progress, filled or not. An error ends the filling only when the
/// session or the database fails.
///
/// Called by: `panels/views/evaluation_plan/fill.rs`: `fill`.
/// Calls: `objects/session/progress/progress_ai/lock.rs`: `lock_progress`;
/// `objects/evaluation/work_unit/work_unit_ai/filling.rs`: `fill_work_unit`;
/// `objects/session/progress/progress_ai/step.rs`: `record_step`.
pub async fn fill_plan(
    database: &Mutex<Option<UserDatabase>>,
    progress: &Mutex<Progress>,
    task: FillTask,
    plan: i64,
    units: &[i64],
) -> Result<(), AiError> {
    ensure!(lock_progress(progress)?.running, AiError::Refused("the filling was not begun".to_owned()));

    for unit in units {
        if lock_progress(progress)?.stop {
            break;
        }

        let (label, filled, notes) = fill_work_unit(database, progress, task, plan, *unit).await?;

        record_step(&mut *lock_progress(progress)?, label, filled, None, notes);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the run: the work units in the order of the work, those of
    //! the user passed over; the inputs missing; a whole answer written.

    use super::*;

    use crate::objects::error::DomainError;
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{
        create_evaluation, read_evaluation_plan, update_evaluation_plan,
    };
    use crate::objects::evaluation::work_unit::work_unit_ai::assessment::{assessed, prepared_assessment};
    use crate::objects::evaluation::work_unit::work_unit_model::Verdict;
    use crate::objects::evaluation::work_unit::work_unit_store::read_work_unit;
    use crate::test_support::database::database;
    use crate::test_support::evaluation::{evaluation, plan_with_planner, write};

    #[test]
    fn the_work_units_come_in_the_order_of_the_work_and_those_of_the_user_are_passed_over() {
        let (_directory, db) = database();
        let (id, first, second) = evaluation(&db);
        let found = preview(&db, id, false).unwrap();

        assert_eq!(found.units, vec![first, second]);
        assert_eq!(found.agents, vec![("CC evaluator".to_owned(), 2)]);
        assert!(found.missing.is_empty());

        let answer = "{\"verdict\": \"pass\", \"analysis\": \"ok\", \"quotes\": []}";

        assert!(assessed(&db, id, first, answer, &[]).unwrap().is_empty());
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

    #[test]
    fn a_plan_without_an_agent_of_the_planner_is_not_filled() {
        let (_directory, db) = database();
        let (plan, _) = plan_with_planner(&db);
        let stored = read_evaluation_plan(&db, plan).unwrap();

        assert_eq!(fill_units(&db, plan).unwrap().len(), 4);
        assert_eq!(
            fill_preview(&db, plan).unwrap(),
            FillPreview { questions: 4, agent: "CC planner".to_owned(), integration: "Ollama · llama3.1".to_owned() }
        );

        update_evaluation_plan(&db, &EvaluationPlan { ai_agent: None, ..stored }).unwrap();

        assert!(matches!(fill_units(&db, plan), Err(AiError::Domain(DomainError::Refused(_)))));
        assert!(matches!(fill_preview(&db, plan), Err(AiError::Domain(DomainError::Refused(_)))));
    }
}
