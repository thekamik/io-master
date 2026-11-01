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
//! is refused by its store.

use crate::infrastructure::clock::now;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::{EvaluationPlan, PlanState};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{
    create_evaluation, read_evaluation_plan, update_evaluation_plan,
};
use crate::objects::evaluation::work_unit::work_unit_model::WorkUnit;
use crate::objects::text::{MAX_NAME, checked_text};

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
/// or a plan its store does not copy.
///
/// Called by: `panels/views/evaluation/evaluations.rs`: `evaluation_create`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/ai/agent/agent_store.rs`:
/// `read_agent`; `infrastructure/clock.rs`: `now`; `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`:
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
/// Called by: [`change_agent`]; `objects/evaluation/product_document/product_document_rules.rs`: `give_document`;
/// `objects/evaluation/lab_proof/lab_proof_rules.rs`: `give_proof`;
/// `objects/evaluation/work_unit/work_unit_rules/results.rs`:
/// `save_results`; `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`:
/// `units_to_assess`, `preview`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`.
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
/// Calls: [`evaluation_in_progress`]; `objects/ai/agent/agent_store.rs`: `read_agent`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `update_evaluation_plan`.
pub fn change_agent(db: &UserDatabase, evaluation: i64, agent: i64) -> Result<(), DomainError> {
    let stored = evaluation_in_progress(db, evaluation)?;

    read_agent(db, agent)?;

    Ok(update_evaluation_plan(db, &EvaluationPlan { ai_agent: Some(agent), ..stored })?)
}

/// The proofs of the laboratory the work units `units` need, each once, in
/// the order of the work units (`[6, 9]`).
///
/// Called by: `objects/evaluation/lab_proof/lab_proof_rules.rs`: `give_proof`; `panels/logic/evaluation/inputs.rs`:
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

    use crate::objects::ai::agent::agent_model::Agent;
    use crate::objects::ai::agent::agent_store::create_agent;
    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::create_evaluation_plan;
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
