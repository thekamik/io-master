//! The rules of a work unit (`WorkUnit`) of a plan in planning: the
//! parameters the planner gives it, written with the cross references of
//! its plan marked again (`../evaluation_plan_rules/order.rs`: `check_order`); a new
//! place of a proof of the laboratory, which makes it a functional test.
//!
//! ```text
//! the planner, or the AI for the planner ──WorkUnitParameters──► update_work_unit_parameters
//!     ──► plan_in_planning ──► update_work_unit ──► check_order
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::ai::agent_roles::agent_roles_rules::evaluator_of;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::order::check_order;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::plan_in_planning;
use crate::objects::evaluation::lab_proof::lab_proof_model::LabProof;
use crate::objects::evaluation::lab_proof::lab_proof_store::create_lab_proof;
use crate::objects::evaluation::work_unit::work_unit_model::WorkUnit;
use crate::objects::evaluation::work_unit::work_unit_store::{read_work_unit, update_work_unit};
use crate::objects::text::{MAX_NAME, checked_text};

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
/// empty list of proofs makes it a test of the documentation (`None`).
///
/// Input: `plan`, the id of a plan in planning; `unit`, the id of one of its
/// work units; `parameters`, as the planner or the AI gives them, the agent
/// named a stored one.
/// Output: nothing; `DomainError::Refused` for a plan not in planning or a
/// work unit not of it, an error of the database for an agent missing or
/// for links its store refuses.
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_save`
/// (the form of a work unit); `objects/evaluation/work_unit/work_unit_ai/proposal.rs`:
/// `apply_parameters`, `apply_dependencies` (proposed by the AI).
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_in_planning`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_rules/order.rs`: `check_order`;
/// `objects/ai/agent/agent_store.rs`: `read_agent`; `objects/ai/agent_roles/agent_roles_rules.rs`:
/// `evaluator_of`; `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`, `update_work_unit`.
pub fn update_work_unit_parameters(
    db: &UserDatabase,
    plan: i64,
    unit: i64,
    parameters: &WorkUnitParameters,
) -> Result<(), DomainError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), DomainError::Refused("the work unit is not one of the plan".to_owned()));

    if let Some(agent) = parameters.ai_agent {
        // Only an evaluator assesses a work unit (user, 2026-11-02).
        let named = read_agent(db, agent)?;

        evaluator_of(&named.role, &named.name)?;
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

/// Reserves a new place of a proof of the laboratory for the work unit
/// `unit` of the plan `plan`, which becomes a functional test.
///
/// Input: `plan`, the id of a plan in planning; `unit`, the id of one of its
/// work units; `name`, the name of the proof ("results of the scan of the
/// ports").
/// Output: the id of the place (`15`); `DomainError::Refused` for a plan not
/// in planning, a work unit not of it or a name refused.
///
/// Called by: `panels/views/evaluation_plan/work_units.rs`: `work_unit_add_proof`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_rules/planning.rs`: `plan_in_planning`;
/// `objects/text.rs`: `checked_text`; `objects/evaluation/lab_proof/lab_proof_store.rs`: `create_lab_proof`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_unit`, `update_work_unit`.
pub fn add_proof_place(db: &UserDatabase, plan: i64, unit: i64, name: &str) -> Result<i64, DomainError> {
    let stored = plan_in_planning(db, plan)?;

    ensure!(stored.work_units.contains(&unit), DomainError::Refused("the work unit is not one of the plan".to_owned()));

    let place = LabProof {
        id: 0,
        name: checked_text(name, "the name of the proof", MAX_NAME)?,
        proof: None,
        performed: None,
        tester: None,
        equipment: None,
    };
    let id = create_lab_proof(db, &place)?;
    let mut changed = read_work_unit(db, unit)?;

    changed.input_functional.get_or_insert_with(Vec::new).push(id);
    update_work_unit(db, &changed)?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Tests of the parameters of a work unit: the agent it names exists; its
    //! dependencies and documents given, a proof making it a functional test.

    use super::*;

    use crate::objects::ai::agent::agent_store::create_agent;
    use crate::objects::ai::integration::integration_store::create_integration;
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::EvaluationPlan;
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::planning::{
        add_document_place, create_plan, update_plan,
    };
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_rules::work_units::add_work_unit;
    use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::read_evaluation_plan;
    use crate::objects::evaluation::oscal_standard_document::oscal_standard_document_rules::import_standard;
    use crate::test_support::database::{agent, database, integration};

    const NIST_SAMPLE: &str = include_str!("../../../../../oscal/examples/basic-catalog.json");

    /// A plan with the sample of NIST as its standard.
    fn plan_with_standard(db: &UserDatabase) -> (i64, i64) {
        let standard = import_standard(db, NIST_SAMPLE).unwrap();
        let plan = create_plan(db, "Sample", "").unwrap();
        let stored = read_evaluation_plan(db, plan).unwrap();

        update_plan(db, &EvaluationPlan { oscal_documents: vec![standard], ..stored }).unwrap();

        (plan, standard)
    }

    #[test]
    fn a_work_unit_names_an_agent_that_exists() {
        let (_directory, db) = database();
        let (plan, standard) = plan_with_standard(&db);
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

    #[test]
    fn a_work_unit_gets_its_parameters_and_becomes_a_functional_test() {
        let (_directory, db) = database();
        let (plan, standard) = plan_with_standard(&db);
        let first = add_work_unit(&db, plan, standard, "s1.1.1").unwrap();
        let second = add_work_unit(&db, plan, standard, "s1.1.2").unwrap();
        let document = add_document_place(&db, plan, "ST").unwrap();
        let parameters = WorkUnitParameters {
            dependencies: vec![first],
            input_product_documents: vec![document],
            input_functional: Vec::new(),
            ai_agent: None,
            guidance: "Read the ST.".to_owned(),
        };

        update_work_unit_parameters(&db, plan, second, &parameters).unwrap();

        let stored = read_work_unit(&db, second).unwrap();

        assert_eq!(stored.dependencies, vec![first]);
        assert_eq!(stored.input_functional, None);

        let proof = add_proof_place(&db, plan, second, "Scan of the ports").unwrap();

        assert_eq!(read_work_unit(&db, second).unwrap().input_functional, Some(vec![proof]));
    }
}
