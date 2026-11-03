//! The fields of an evaluation as a report writes them (user, 2026-11-03):
//! each field of the model is the value of its marker
//! (`{{evaluation_plan.product}}`); a field that names another object writes
//! its name. Its lists (the standards, the documents, the work units) are
//! the blocks of a template, each element with its own fields.
//!
//! ```text
//! an evaluation ──► evaluation_plan_fields ──► [the values, in the order of EVALUATION_PLAN_FIELDS]
//! ```

use crate::infrastructure::clock::date_of;
use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::{EvaluationPlan, PlanState};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
use crate::objects::evaluation::work_unit::work_unit_rules::fields::verdict_text;

/// The fields of an evaluation a report writes, in the order of
/// [`evaluation_plan_fields`]: every field of the model but its lists.
pub const EVALUATION_PLAN_FIELDS: [&str; 12] = [
    "id",
    "name",
    "description",
    "state",
    "source_plan",
    "product",
    "client",
    "evaluator",
    "started",
    "possible_verdicts",
    "blocked",
    "ai_agent",
];

/// The fields of the evaluation `plan` as a report writes them, in the
/// order of [`EVALUATION_PLAN_FIELDS`]: the name of the plan it was copied
/// from and of its agent, the day it was begun, its verdicts parted by
/// commas, "yes" or "no".
///
/// Input: `db`, the open database of the user; `plan`, an evaluation.
/// Output: its fields (`["12", "Routers", "", "Evaluation", "Routers", "R7", ...]`); an
/// error when it names a missing plan or a missing agent.
///
/// Called by: `objects/evaluation/report_template/report_template_rules/filling.rs`: `filled_report`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`; `infrastructure/clock.rs`: `date_of`;
/// `objects/evaluation/work_unit/work_unit_rules/fields.rs`: `verdict_text`.
pub fn evaluation_plan_fields(db: &UserDatabase, plan: &EvaluationPlan) -> Result<[String; 12], DomainError> {
    let state = match plan.state {
        PlanState::Plan => "Plan",
        PlanState::Evaluation => "Evaluation",
        PlanState::Report => "Report",
    };
    let source_plan = match plan.source_plan {
        Some(source) => read_evaluation_plan(db, source)?.name,
        None => String::new(),
    };
    let agent = match plan.ai_agent {
        Some(agent) => read_agent(db, agent)?.name,
        None => String::new(),
    };
    let mut verdicts = Vec::with_capacity(plan.possible_verdicts.len());

    for verdict in &plan.possible_verdicts {
        verdicts.push(verdict_text(*verdict));
    }

    Ok([
        plan.id.to_string(),
        plan.name.clone(),
        plan.description.clone(),
        state.to_owned(),
        source_plan,
        plan.product.clone(),
        plan.client.clone(),
        plan.evaluator.clone(),
        plan.started.map(date_of).unwrap_or_default(),
        verdicts.join(", "),
        (if plan.blocked { "yes" } else { "no" }).to_owned(),
        agent,
    ])
}

#[cfg(test)]
mod tests {
    //! Tests of the fields of an evaluation.

    use super::*;

    use crate::test_support::database::database;
    use crate::test_support::evaluation::evaluation;

    #[test]
    fn an_evaluation_writes_its_names_and_its_day() {
        let (_directory, db) = database();
        let (id, _, _) = evaluation(&db);
        let fields = evaluation_plan_fields(&db, &read_evaluation_plan(&db, id).unwrap()).unwrap();

        assert_eq!(fields[3], "Evaluation");
        assert_eq!(fields[4], read_evaluation_plan(&db, id).unwrap().name);
        assert_eq!(fields[5], "R7");
        assert_eq!(fields[8], "2026-10-26");
        assert_eq!(fields[9], "Pass, Fail");
        assert_eq!(fields[10], "no");
        assert!(!fields[11].is_empty());
    }
}
