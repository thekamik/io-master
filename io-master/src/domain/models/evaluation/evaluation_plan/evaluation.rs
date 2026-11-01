//! A plan in evaluation (user, 2026-10-30): a plan copied for one product,
//! with the identification ISO/IEC 17025 asks for and the agent that
//! assesses every work unit without an agent of its own; an evaluation in
//! progress, or refused; the proofs its work units need.
//!
//! ```text
//! new_evaluation(plan, product, client, evaluator, agent) ──► create_evaluation: the copy, state Evaluation
//! change_agent(evaluation, agent)                         ──► update_evaluation_plan: its ai_agent
//! the panels, the AI ──► evaluation_in_progress (the evaluation, or refused)
//! ```
//!
//! The plan stays as it was; a blocked plan, or one with a cross reference,
//! is refused by `db/`.

use crate::clock::now;
use crate::db::models::ai::agent::read_agent;
use crate::db::models::evaluation::evaluation_plan::{create_evaluation, read_evaluation_plan, update_evaluation_plan};
use crate::db::storage::user_database::UserDatabase;
use crate::domain::error::DomainError;
use crate::domain::text::{MAX_NAME, checked_text};
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::model::evaluation::work_unit::WorkUnit;

/// Begins the evaluation of a product on a copy of the plan `plan`, with
/// the agent `agent` for the work units without an agent of their own.
///
/// Input:
/// - `plan`: a plan in planning, not blocked.
/// - `identification`: the product, the client and the evaluator
///   (`["Router R7", "ACME", "K. K."]`), each given, at most [`MAX_NAME`]
///   characters.
/// - `agent`: an agent that exists.
///
/// Output: the id of the evaluation (12); `DomainError::Refused` for a field
/// missing or too long, `DomainError::Db` for an agent that does not exist
/// or a plan `db/` does not copy.
///
/// Called by: `panels/views/evaluation/evaluations.rs`: `evaluation_create`.
/// Calls: `domain/text.rs`: `checked_text`; `db/models/ai/agent.rs`:
/// `read_agent`; `clock.rs`: `now`; `db/models/evaluation/evaluation_plan.rs`:
/// `create_evaluation`.
pub fn new_evaluation(db: &UserDatabase, plan: i64, identification: [&str; 3], agent: i64) -> Result<i64, DomainError> {
    let [product, client, evaluator] = identification;
    let product = checked_text(product, "the product", MAX_NAME)?;
    let client = checked_text(client, "the client", MAX_NAME)?;
    let evaluator = checked_text(evaluator, "the evaluator", MAX_NAME)?;

    read_agent(db, agent)?;

    let started = now().map_err(|error| DomainError::Refused(error.to_string()))?;

    Ok(create_evaluation(db, plan, &product, &client, &evaluator, started, Some(agent))?)
}

/// The evaluation `id`; refused unless it is being carried out (state
/// `Evaluation`).
///
/// Called by: [`change_agent`]; `../product_document.rs`: `give_document`;
/// `../lab_proof.rs`: `give_proof`; `domain/models/evaluation/work_unit/results.rs`:
/// `save_results`; `ai/models/evaluation/evaluation_plan.rs`:
/// `units_to_assess`, `preview`.
/// Calls: `db/models/evaluation/evaluation_plan.rs`: `read_evaluation_plan`.
pub fn evaluation_in_progress(db: &UserDatabase, id: i64) -> Result<EvaluationPlan, DomainError> {
    let evaluation = read_evaluation_plan(db, id)?;

    ensure!(
        evaluation.state == PlanState::Evaluation,
        DomainError::Refused(format!("{} is no evaluation in progress", evaluation.name))
    );

    Ok(evaluation)
}

/// Gives the evaluation `evaluation` the agent `agent` for its work units
/// without an agent of their own.
///
/// Called by: `panels/views/evaluation/evaluations.rs`: `evaluation_set_agent`.
/// Calls: [`evaluation_in_progress`]; `db/models/ai/agent.rs`: `read_agent`;
/// `db/models/evaluation/evaluation_plan.rs`: `update_evaluation_plan`.
pub fn change_agent(db: &UserDatabase, evaluation: i64, agent: i64) -> Result<(), DomainError> {
    let stored = evaluation_in_progress(db, evaluation)?;

    read_agent(db, agent)?;

    Ok(update_evaluation_plan(db, &EvaluationPlan { ai_agent: Some(agent), ..stored })?)
}

/// The proofs of the laboratory the work units `units` need, each once, in
/// the order of the work units (`[6, 9]`).
///
/// Called by: `../lab_proof.rs`: `give_proof`; `panels/logic/evaluation/inputs.rs`:
/// `inputs`.
pub fn needed_proofs(units: &[WorkUnit]) -> Vec<i64> {
    let mut proofs = Vec::new();

    for unit in units {
        for proof in unit.input_functional.as_deref().unwrap_or_default() {
            if !proofs.contains(proof) {
                proofs.push(*proof);
            }
        }
    }

    proofs
}

#[cfg(test)]
mod tests {
    //! Tests of the evaluations: begun on a copy of a plan with their agent,
    //! the agent changed; what does not hold together is refused.

    use super::*;

    use crate::db::models::ai::agent::create_agent;
    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::evaluation::evaluation_plan::create_evaluation_plan;
    use crate::model::ai::agent::Agent;
    use crate::test_support::database::{agent, database, integration, plan};

    #[test]
    fn an_evaluation_begins_on_a_copy_of_a_plan_with_its_agent() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let general = create_agent(&db, &agent(local)).unwrap();
        let other = create_agent(&db, &Agent { name: "Specialist".to_owned(), ..agent(local) }).unwrap();
        let source = create_evaluation_plan(&db, &plan(&db)).unwrap();

        assert!(matches!(new_evaluation(&db, source, [" ", "ACME", "K. K."], general), Err(DomainError::Refused(_))));
        assert!(matches!(new_evaluation(&db, source, ["R7", "ACME", "K. K."], 999), Err(DomainError::Db(_))));

        let id = new_evaluation(&db, source, [" Router R7 ", "ACME", "K. K."], general).unwrap();
        let evaluation = read_evaluation_plan(&db, id).unwrap();

        assert_eq!((evaluation.state, evaluation.product.as_str()), (PlanState::Evaluation, "Router R7"));
        assert_eq!(evaluation.ai_agent, Some(general));
        assert!(evaluation.started.is_some_and(|started| started > 0));

        change_agent(&db, id, other).unwrap();
        assert_eq!(read_evaluation_plan(&db, id).unwrap().ai_agent, Some(other));
        assert!(matches!(change_agent(&db, source, other), Err(DomainError::Refused(_))));
    }
}
