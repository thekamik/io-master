//! The life of a plan (`EvaluationPlan`) and its data: a new plan, its data
//! changed by the planner (its name, description, standards, places of the
//! documents of the product, verdicts, agent), a new place of a document;
//! a plan in planning, or refused. A plan is
//! changed only while it is planned (state `Plan`); an evaluation is a copy
//! of it.
//!
//! ```text
//! the planner ──► create_plan ──► create_evaluation_plan
//!             ──► update_plan ──► check_planning (before and after) ──► update_evaluation_plan
//! the panels, the AI ──► plan_in_planning, plan_to_fill (the plan in planning, or refused)
//! ```

use crate::infrastructure::storage::user_database::UserDatabase;
use crate::objects::ai::agent::agent_store::read_agent;
use crate::objects::error::DomainError;
use crate::objects::evaluation::evaluation_plan::evaluation_plan_model::{EvaluationPlan, PlanState};
use crate::objects::evaluation::evaluation_plan::evaluation_plan_store::{
    create_evaluation_plan, read_evaluation_plan, update_evaluation_plan,
};
use crate::objects::evaluation::product_document::product_document_model::ProductDocument;
use crate::objects::evaluation::product_document::product_document_store::create_product_document;
use crate::objects::evaluation::work_unit::work_unit_model::Verdict;
use crate::objects::evaluation::work_unit::work_unit_store::read_work_units;
use crate::objects::text::{MAX_NAME, checked_text};

/// Refuses a plan that is no more in planning, and one without a name.
///
/// Input: `plan`, a plan as stored or as changed.
/// Output: nothing; `DomainError::Refused` with the reason.
///
/// Called by: [`plan_in_planning`], [`update_plan`], [`add_document_place`].
pub fn check_planning(plan: &EvaluationPlan) -> Result<(), DomainError> {
    ensure!(
        plan.state == PlanState::Plan,
        DomainError::Refused("only a plan in planning is changed; an evaluation keeps its plan".to_owned())
    );
    ensure!(!plan.name.trim().is_empty(), DomainError::Refused("a plan without a name".to_owned()));

    Ok(())
}

/// The plan `plan`, refused unless it is in planning.
///
/// Input: `plan`, the id of a stored plan.
/// Output: the plan; the refusals of [`check_planning`], an error of the
/// database for a plan missing.
///
/// Called by: [`plan_to_fill`]; `objects/evaluation/evaluation_plan/evaluation_plan_rules/work_units.rs`:
/// `add_work_unit`, `add_all_work_units`, `remove_work_unit`;
/// `objects/evaluation/work_unit/work_unit_rules/parameters.rs`: `update_work_unit_parameters`,
/// `add_proof_place`.
/// Calls: `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`;
/// [`check_planning`].
pub fn plan_in_planning(db: &UserDatabase, plan: i64) -> Result<EvaluationPlan, DomainError> {
    let stored = read_evaluation_plan(db, plan)?;

    check_planning(&stored)?;

    Ok(stored)
}

/// The plan `plan` to fill with the AI ("Fill with AI").
///
/// Input: `plan`, the id of a stored plan.
/// Output: the plan and the id of its agent of the planner (`(plan, 2)`);
/// refused unless it is in planning and names an agent of the planner.
///
/// Called by: `objects/evaluation/evaluation_plan/evaluation_plan_ai.rs`: `fill_units`,
/// `fill_preview`; `objects/evaluation/work_unit/work_unit_ai/proposal.rs`:
/// `question_of`, `parameters_question`, `apply_parameters`, `apply_dependencies`.
/// Calls: [`plan_in_planning`].
pub fn plan_to_fill(db: &UserDatabase, plan: i64) -> Result<(EvaluationPlan, i64), DomainError> {
    let stored = plan_in_planning(db, plan)?;

    let Some(agent) = stored.ai_agent else {
        return Err(DomainError::Refused("the plan has no agent of the planner: choose one in its data".to_owned()));
    };

    Ok((stored, agent))
}

/// Makes a new plan in planning, without standards or work units yet, with
/// the verdicts of Common Criteria (`Pass`, `Fail`).
///
/// Input: `name`, the name typed ("CC EAL4+ router"); `description`, may be
/// empty.
/// Output: the id of the new plan (`1`); `DomainError::Refused` for a name
/// refused, an error of the database.
///
/// Called by: `panels/views/evaluation_plan/plans.rs`: `plan_create`.
/// Calls: `objects/text.rs`: `checked_text`; `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`:
/// `create_evaluation_plan`.
pub fn create_plan(db: &UserDatabase, name: &str, description: &str) -> Result<i64, DomainError> {
    let name = checked_text(name, "the name of the plan", MAX_NAME)?;

    let plan = EvaluationPlan {
        id: 0,
        name,
        description: description.trim().to_owned(),
        state: PlanState::Plan,
        source_plan: None,
        product: String::new(),
        client: String::new(),
        evaluator: String::new(),
        started: None,
        oscal_documents: Vec::new(),
        product_documents: Vec::new(),
        possible_verdicts: vec![Verdict::Pass, Verdict::Fail],
        work_units: Vec::new(),
        blocked: false,
        ai_agent: None,
    };

    Ok(create_evaluation_plan(db, &plan)?)
}

/// Writes the data of the plan `plan.id` the planner changed: its name,
/// description, standards, places of documents, verdicts and the agent of
/// the planner. Its state, its work units and `blocked` stay as stored.
///
/// Input: `plan`, the plan as changed, its id that of a stored plan.
/// Output: nothing; `DomainError::Refused` when the plan is no more in
/// planning or has no name, or a work unit of the plan still uses a standard
/// or a document the plan would lose; an error of the database for an agent
/// missing.
///
/// Called by: `panels/views/evaluation_plan/plans.rs`: `plan_save`.
/// Calls: [`check_planning`]; `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`:
/// `read_evaluation_plan`, `update_evaluation_plan`; `objects/ai/agent/agent_store.rs`: `read_agent`;
/// `objects/evaluation/work_unit/work_unit_store.rs`: `read_work_units`.
pub fn update_plan(db: &UserDatabase, plan: &EvaluationPlan) -> Result<(), DomainError> {
    let stored = read_evaluation_plan(db, plan.id)?;

    check_planning(&stored)?;

    let changed = EvaluationPlan {
        name: plan.name.trim().to_owned(),
        description: plan.description.trim().to_owned(),
        oscal_documents: plan.oscal_documents.clone(),
        product_documents: plan.product_documents.clone(),
        possible_verdicts: plan.possible_verdicts.clone(),
        ai_agent: plan.ai_agent,
        ..stored
    };

    check_planning(&changed)?;

    if let Some(agent) = changed.ai_agent {
        read_agent(db, agent)?;
    }

    for unit in read_work_units(db, plan.id)? {
        ensure!(
            changed.oscal_documents.contains(&unit.reference),
            DomainError::Refused(format!("the work unit {} still uses the standard taken out", unit.label))
        );

        for document in &unit.input_product_documents {
            ensure!(
                changed.product_documents.contains(document),
                DomainError::Refused(format!("the work unit {} still uses the document taken out", unit.label))
            );
        }
    }

    Ok(update_evaluation_plan(db, &changed)?)
}

/// Reserves a new place of a document of the product in the plan `plan`.
///
/// Input: `plan`, the id of a plan in planning; `name`, the name of the
/// document ("ST", "IXIT").
/// Output: the id of the place (`9`); the refusals of [`check_planning`] or
/// for a name refused, an error of the database.
///
/// Called by: `panels/views/evaluation_plan/plans.rs`: `plan_add_document`.
/// Calls: [`check_planning`]; `objects/text.rs`: `checked_text`;
/// `objects/evaluation/evaluation_plan/evaluation_plan_store.rs`: `read_evaluation_plan`, `update_evaluation_plan`;
/// `objects/evaluation/product_document/product_document_store.rs`: `create_product_document`.
pub fn add_document_place(db: &UserDatabase, plan: i64, name: &str) -> Result<i64, DomainError> {
    let mut stored = read_evaluation_plan(db, plan)?;

    check_planning(&stored)?;

    let place = ProductDocument {
        id: 0,
        name: checked_text(name, "the name of the document", MAX_NAME)?,
        content: None,
        version: None,
    };
    let id = create_product_document(db, &place)?;

    stored.product_documents.push(id);
    update_evaluation_plan(db, &stored)?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Tests of the plans in planning: made, changed, guarded.

    use super::*;

    use crate::objects::evaluation::work_unit::work_unit_store::create_work_unit;
    use crate::test_support::database::{database, plan, unit};

    #[test]
    fn a_new_plan_has_the_verdicts_of_common_criteria_and_takes_its_data() {
        let (_directory, db) = database();
        let id = create_plan(&db, "  Routers  ", "EN 303 645").unwrap();
        let place = add_document_place(&db, id, "ST").unwrap();
        let mut stored = read_evaluation_plan(&db, id).unwrap();

        assert_eq!(stored.name, "Routers");
        assert_eq!(stored.possible_verdicts, vec![Verdict::Pass, Verdict::Fail]);
        assert_eq!(stored.product_documents, vec![place]);

        stored.possible_verdicts.push(Verdict::Inconclusive);
        stored.state = PlanState::Report;
        update_plan(&db, &stored).unwrap();

        let read = read_evaluation_plan(&db, id).unwrap();

        assert_eq!(read.possible_verdicts.len(), 3);
        assert_eq!(read.state, PlanState::Plan);
        assert!(matches!(update_plan(&db, &EvaluationPlan { ai_agent: Some(99), ..read }), Err(DomainError::Db(_))));
    }

    #[test]
    fn a_standard_or_a_document_used_by_a_work_unit_stays() {
        let (_directory, db) = database();
        let routers = plan(&db);
        let id = create_evaluation_plan(&db, &routers).unwrap();

        create_work_unit(&db, id, &unit(&routers, "ASE_INT.1-1")).unwrap();

        let stored = read_evaluation_plan(&db, id).unwrap();

        assert!(matches!(
            update_plan(&db, &EvaluationPlan { oscal_documents: Vec::new(), ..stored.clone() }),
            Err(DomainError::Refused(_))
        ));
        assert!(matches!(
            update_plan(&db, &EvaluationPlan { product_documents: Vec::new(), ..stored.clone() }),
            Err(DomainError::Refused(_))
        ));
        assert!(matches!(create_plan(&db, " ", ""), Err(DomainError::Refused(_))));
    }
}
