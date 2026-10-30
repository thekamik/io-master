//! The evaluations (user, 2026-10-30): a plan copied for one product, with
//! the identification ISO/IEC 17025 asks for and the agent that assesses
//! every work unit without an agent of its own.
//!
//! ```text
//! new_evaluation(plan, product, client, evaluator, agent) ──► create_evaluation: the copy, state Evaluation
//! change_agent(evaluation, agent)                         ──► update_evaluation_plan: its ai_agent
//! ```
//!
//! The plan stays as it was; a blocked plan, or one with a cross reference,
//! is refused by `db/`.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::ai::agent::read_agent;
use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::{create_evaluation, read_evaluation_plan, update_evaluation_plan};
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::panels::logic::evaluation::error::EvaluationError;

/// The longest product, client or evaluator, in characters.
const MAX_FIELD: usize = 200;

/// The time now, in seconds since 1970 (UTC).
///
/// Called by: [`new_evaluation`].
fn now() -> Result<i64, EvaluationError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| EvaluationError::Refused(format!("the clock: {error}")))?
        .as_secs();

    i64::try_from(seconds).map_err(|error| EvaluationError::Refused(format!("the clock: {error}")))
}

/// The text `text` trimmed; refused when it is empty or over [`MAX_FIELD`].
///
/// Called by: [`new_evaluation`].
fn checked_field(text: &str, what: &str) -> Result<String, EvaluationError> {
    let text = text.trim();

    ensure!(!text.is_empty(), EvaluationError::Refused(format!("an evaluation without {what}")));
    ensure!(text.chars().count() <= MAX_FIELD, EvaluationError::Refused(format!("{what} over {MAX_FIELD} characters")));

    Ok(text.to_owned())
}

/// The evaluation `id`; refused unless it is being carried out (state
/// `Evaluation`).
///
/// Called by: [`change_agent`]; `inputs.rs` (a document or a proof given).
/// Calls: `read_evaluation_plan`.
pub fn evaluation_in_progress(db: &Database, id: i64) -> Result<EvaluationPlan, EvaluationError> {
    let evaluation = read_evaluation_plan(db, id)?;

    ensure!(
        evaluation.state == PlanState::Evaluation,
        EvaluationError::Refused(format!("{} is no evaluation in progress", evaluation.name))
    );

    Ok(evaluation)
}

/// Begins the evaluation of `product` for `client` by `evaluator` on a copy
/// of the plan `plan`, with the agent `agent` for the work units without an
/// agent of their own; gives its id.
///
/// Called by: `panels/views/evaluation/evaluations.rs` (`evaluation_create`).
/// Calls: [`checked_field`], `read_agent`, [`now`], `create_evaluation`.
pub fn new_evaluation(db: &Database, plan: i64, identification: [&str; 3], agent: i64) -> Result<i64, EvaluationError> {
    let [product, client, evaluator] = identification;
    let product = checked_field(product, "a product")?;
    let client = checked_field(client, "a client")?;
    let evaluator = checked_field(evaluator, "an evaluator")?;

    read_agent(db, agent)?;

    Ok(create_evaluation(db, plan, &product, &client, &evaluator, now()?, Some(agent))?)
}

/// Gives the evaluation `evaluation` the agent `agent` for its work units
/// without an agent of their own.
///
/// Called by: `panels/views/evaluation/evaluations.rs` (`evaluation_set_agent`).
/// Calls: [`evaluation_in_progress`], `read_agent`, `update_evaluation_plan`.
pub fn change_agent(db: &Database, evaluation: i64, agent: i64) -> Result<(), EvaluationError> {
    let stored = evaluation_in_progress(db, evaluation)?;

    read_agent(db, agent)?;

    Ok(update_evaluation_plan(db, &EvaluationPlan { ai_agent: Some(agent), ..stored })?)
}

#[cfg(test)]
mod tests {
    //! Tests of the evaluations: begun on a copy of a plan with their agent,
    //! the agent changed; what does not hold together is refused.

    use super::*;

    use crate::db::ai::agent::create_agent;
    use crate::db::ai::integration::create_integration;
    use crate::db::evaluation::evaluation_plan::create_evaluation_plan;
    use crate::model::ai::agent::Agent;
    use crate::test_support::database::{agent, database, integration, plan};

    #[test]
    fn an_evaluation_begins_on_a_copy_of_a_plan_with_its_agent() {
        let (_directory, db) = database();
        let local = create_integration(&db, &integration()).unwrap();
        let general = create_agent(&db, &agent(local)).unwrap();
        let other = create_agent(&db, &Agent { name: "Specialist".to_owned(), ..agent(local) }).unwrap();
        let source = create_evaluation_plan(&db, &plan(&db)).unwrap();

        assert!(matches!(
            new_evaluation(&db, source, [" ", "ACME", "K. K."], general),
            Err(EvaluationError::Refused(_))
        ));
        assert!(matches!(new_evaluation(&db, source, ["R7", "ACME", "K. K."], 999), Err(EvaluationError::Db(_))));

        let id = new_evaluation(&db, source, [" Router R7 ", "ACME", "K. K."], general).unwrap();
        let evaluation = read_evaluation_plan(&db, id).unwrap();

        assert_eq!((evaluation.state, evaluation.product.as_str()), (PlanState::Evaluation, "Router R7"));
        assert_eq!(evaluation.ai_agent, Some(general));
        assert!(evaluation.started.is_some_and(|started| started > 0));

        change_agent(&db, id, other).unwrap();
        assert_eq!(read_evaluation_plan(&db, id).unwrap().ai_agent, Some(other));
        assert!(matches!(change_agent(&db, source, other), Err(EvaluationError::Refused(_))));
    }
}
