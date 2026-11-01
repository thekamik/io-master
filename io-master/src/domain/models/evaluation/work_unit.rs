//! The rules of a work unit (`WorkUnit`) of a plan in planning: the
//! parameters the planner gives it, written with the cross references of
//! its plan marked again (`evaluation_plan.rs`: `check_order`).
//!
//! ```text
//! the planner, or the AI for the planner ──WorkUnitParameters──► update_work_unit_parameters
//!     ──► plan_in_planning ──► update_work_unit ──► check_order
//! ```

use crate::db::models::ai::agent::read_agent;
use crate::db::models::evaluation::work_unit::{read_work_unit, update_work_unit};
use crate::db::storage::user_database::UserDatabase;
use crate::domain::error::DomainError;
use crate::domain::models::evaluation::evaluation_plan::{check_order, plan_in_planning};
use crate::model::evaluation::work_unit::WorkUnit;

/// What the planner gives a work unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkUnitParameters {
    /// The work units of the plan it depends on.
    pub dependencies: Vec<i64>,
    /// The documents of the product (places of the plan) it needs.
    pub input_product_documents: Vec<i64>,
    /// The proofs of the laboratory it needs; empty: a test of the
    /// documentation.
    pub input_functional: Vec<i64>,
    /// The agent that assesses it; `None`: none.
    pub ai_agent: Option<i64>,
    /// The guidance of the standard, as the planner corrected it.
    pub guidance: String,
}

/// Gives the work unit `unit` of the plan `plan` in planning the
/// `parameters` of the planner, and marks the cross references again. An
/// empty list of proofs makes it a test of the documentation (`None`); the
/// agent named must exist (`ai_agent`, the agent that will work on it).
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_save`
/// (the form of a work unit); `ai/models/evaluation/work_unit/proposal.rs`:
/// `apply_parameters`, `apply_dependencies` (proposed by the AI).
/// Calls: `evaluation_plan.rs`: `plan_in_planning`, `check_order`;
/// `db/models/ai/agent.rs`: `read_agent`; `db/models/evaluation/work_unit.rs`:
/// `read_work_unit`, `update_work_unit`.
pub fn update_work_unit_parameters(
    db: &UserDatabase,
    plan: i64,
    unit: i64,
    parameters: &WorkUnitParameters,
) -> Result<(), DomainError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), DomainError::Refused("the work unit is not one of the plan".to_owned()));

    if let Some(agent) = parameters.ai_agent {
        read_agent(db, agent)?;
    }

    let changed = WorkUnit {
        dependencies: parameters.dependencies.clone(),
        input_product_documents: parameters.input_product_documents.clone(),
        input_functional: if parameters.input_functional.is_empty() {
            None
        } else {
            Some(parameters.input_functional.clone())
        },
        ai_agent: parameters.ai_agent,
        guidance: parameters.guidance.clone(),
        ..read_work_unit(db, unit)?
    };

    update_work_unit(db, &changed)?;
    check_order(db, plan)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests of the parameters of a work unit: the agent it names exists.

    use super::*;

    use crate::db::models::ai::agent::create_agent;
    use crate::db::models::ai::integration::create_integration;
    use crate::db::models::evaluation::evaluation_plan::read_evaluation_plan;
    use crate::model::evaluation::evaluation_plan::EvaluationPlan;
    use crate::panels::logic::evaluation_plan::plans::{create_plan, update_plan};
    use crate::panels::logic::evaluation_plan::standards::import_standard;
    use crate::panels::logic::evaluation_plan::work_units::add_work_unit;
    use crate::test_support::database::{agent, database, integration};

    const NIST_SAMPLE: &str = include_str!("../../../../oscal/examples/basic-catalog.json");

    #[test]
    fn a_work_unit_names_an_agent_that_exists() {
        let (_directory, db) = database();
        let standard = import_standard(&db, NIST_SAMPLE).unwrap();
        let plan = create_plan(&db, "Sample", "").unwrap();
        let stored = read_evaluation_plan(&db, plan).unwrap();

        update_plan(&db, &EvaluationPlan { oscal_documents: vec![standard], ..stored }).unwrap();

        let unit = add_work_unit(&db, plan, standard, "s1.1.1").unwrap();
        let integration = create_integration(&db, &integration()).unwrap();
        let worker = create_agent(&db, &agent(integration)).unwrap();
        let named = |ai_agent: Option<i64>| WorkUnitParameters {
            dependencies: Vec::new(),
            input_product_documents: Vec::new(),
            input_functional: Vec::new(),
            ai_agent,
            guidance: String::new(),
        };

        update_work_unit_parameters(&db, plan, unit, &named(Some(worker))).unwrap();

        assert_eq!(read_work_unit(&db, unit).unwrap().ai_agent, Some(worker));
        assert!(matches!(
            update_work_unit_parameters(&db, plan, unit, &named(Some(worker + 1))),
            Err(DomainError::Db(_))
        ));
    }
}
