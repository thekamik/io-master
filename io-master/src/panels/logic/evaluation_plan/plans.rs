//! The plans in planning: a new plan, its data changed by the planner
//! (its name, description, standards, places of the documents of the
//! product, verdicts), a new place of a document. A plan is changed here
//! only while it is planned (state `Plan`); an evaluation is a copy of it.

use crate::db::ai::agent::read_agent;
use crate::db::database::Database;
use crate::db::evaluation::evaluation_plan::{create_evaluation_plan, read_evaluation_plan, update_evaluation_plan};
use crate::db::evaluation::product_document::create_product_document;
use crate::db::evaluation::work_unit::read_work_units;
use crate::model::evaluation::evaluation_plan::{EvaluationPlan, PlanState};
use crate::model::evaluation::product_document::ProductDocument;
use crate::model::evaluation::work_unit::Verdict;
use crate::panels::logic::evaluation_plan::error::PlanError;

/// Refuses a plan that is no more in planning, and one without a name.
///
/// Called by: [`update_plan`], [`add_document_place`]; `work_units.rs`.
pub(in crate::panels::logic::evaluation_plan) fn check_planning(plan: &EvaluationPlan) -> Result<(), PlanError> {
    ensure!(
        plan.state == PlanState::Plan,
        PlanError::Refused("only a plan in planning is changed; an evaluation keeps its plan".to_owned())
    );
    ensure!(!plan.name.trim().is_empty(), PlanError::Refused("a plan without a name".to_owned()));

    Ok(())
}

/// Makes a new plan in planning, without standards or work units yet, with
/// the verdicts of Common Criteria (`Pass`, `Fail`). Gives its id.
///
/// Called by: `panels/views/evaluation_plan/` (a new plan).
/// Calls: `create_evaluation_plan`.
pub fn create_plan(db: &Database, name: &str, description: &str) -> Result<i64, PlanError> {
    let name = name.trim();

    ensure!(!name.is_empty(), PlanError::Refused("a plan without a name".to_owned()));

    let plan = EvaluationPlan {
        id: 0,
        name: name.to_owned(),
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
/// Refused when the plan is no more in planning, when a work unit of the
/// plan still uses a standard or a document the plan would lose, or when
/// the agent does not exist.
///
/// Called by: `panels/views/evaluation_plan/` (the data of a plan).
/// Calls: `read_evaluation_plan`, [`check_planning`], `read_agent`,
/// `read_work_units`, `update_evaluation_plan`.
pub fn update_plan(db: &Database, plan: &EvaluationPlan) -> Result<(), PlanError> {
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
            PlanError::Refused(format!("the work unit {} still uses the standard taken out", unit.label))
        );

        for document in &unit.input_product_documents {
            ensure!(
                changed.product_documents.contains(document),
                PlanError::Refused(format!("the work unit {} still uses the document taken out", unit.label))
            );
        }
    }

    Ok(update_evaluation_plan(db, &changed)?)
}

/// Reserves a new place of a document of the product (`name`: "ST",
/// "IXIT") in the plan `plan`. Gives the id of the place.
///
/// Called by: `panels/views/evaluation_plan/` (a new document of a plan).
/// Calls: `read_evaluation_plan`, [`check_planning`],
/// `create_product_document`, `update_evaluation_plan`.
pub fn add_document_place(db: &Database, plan: i64, name: &str) -> Result<i64, PlanError> {
    let mut stored = read_evaluation_plan(db, plan)?;

    check_planning(&stored)?;
    ensure!(!name.trim().is_empty(), PlanError::Refused("a document without a name".to_owned()));

    let place = ProductDocument { id: 0, name: name.trim().to_owned(), content: None, version: None };
    let id = create_product_document(db, &place)?;

    stored.product_documents.push(id);
    update_evaluation_plan(db, &stored)?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    //! Tests of the plans in planning: made, changed, guarded.

    use super::*;

    use crate::db::evaluation::work_unit::create_work_unit;
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
        assert!(matches!(update_plan(&db, &EvaluationPlan { ai_agent: Some(99), ..read }), Err(PlanError::Db(_))));
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
            Err(PlanError::Refused(_))
        ));
        assert!(matches!(
            update_plan(&db, &EvaluationPlan { product_documents: Vec::new(), ..stored.clone() }),
            Err(PlanError::Refused(_))
        ));
        assert!(matches!(create_plan(&db, " ", ""), Err(PlanError::Refused(_))));
    }
}
